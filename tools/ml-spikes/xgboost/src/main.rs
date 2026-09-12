use std::{error::Error, path::PathBuf, time::Instant};
use xgb::{Booster, DMatrix, parameters::BoosterParameters};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("output"));
    std::fs::create_dir_all(&output)?;
    // This measures loadability and a real fit/save/load/predict cycle, not
    // generalisation. The repeated, separable rows make failure conspicuous.
    let mut features = Vec::new();
    let mut labels = Vec::new();
    for row in 0..128 {
        let class = (row % 2) as f32;
        features.extend([class, (row % 7) as f32]);
        labels.push(class);
    }
    let started = Instant::now();
    let mut train = DMatrix::from_dense(&features, labels.len())?;
    train.set_labels(&labels)?;
    let mut model = Booster::new_with_cached_dmats(&BoosterParameters::default(), &[&train])?;
    for (name, value) in [
        ("objective", "binary:logistic"),
        ("tree_method", "hist"),
        ("max_depth", "2"),
        ("eta", "0.3"),
        ("seed", "42"),
        ("nthread", "1"),
    ] {
        model.set_param(name, value)?;
    }
    for round in 0..20 {
        model.update(&train, round)?;
    }
    let fit_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let prediction = model.predict(&train)?;
    assert_eq!(prediction.len(), labels.len());
    for (p, y) in prediction.iter().zip(&labels) {
        assert!(p.is_finite() && (0.0..=1.0).contains(p));
        assert_eq!(*p >= 0.5, *y == 1.0);
    }
    let path = output.join("binary.json");
    model.save(&path)?;
    let restored = Booster::load(&path)?;
    let reloaded = restored.predict(&train)?;
    assert_eq!(prediction, reloaded, "JSON round trip changed predictions");
    // Missing input exercises the native default branch rather than replacing
    // missing values in the calling code.
    let missing = DMatrix::from_dense(&[f32::NAN, 1.0], 1)?;
    assert!(restored.predict(&missing)?[0].is_finite());
    println!("fit_ms={fit_ms:.3}");
    println!("rows=128 rounds=20 threads=1 training_accuracy=1.0");
    println!("json_roundtrip_exact=true missing_input_finite=true");
    println!("first_predictions={:?}", &prediction[..4]);
    println!("model_bytes={}", std::fs::metadata(&path)?.len());
    Ok(())
}
