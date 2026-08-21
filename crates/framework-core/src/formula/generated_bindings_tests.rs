//! GENERATED FILE — do not edit by hand.
//! Regenerate with: python3 tools/generate_expr_bindings.py
//!
//! Execution smoke tests for the generated Polars expression bindings in
//! `generated_expr_bindings.rs`. Each formula below is added as a computed
//! column against a small fixed fixture frame and must parse, compile, and
//! execute (collect) without error. This does not assert the *value* each
//! method produces — only that the generated dispatch code is wired
//! correctly end to end. See `tools/generate_expr_bindings.py`'s
//! `SMOKE_TEST_SKIP` / `SMOKE_TEST_NAMESPACE_SKIP` for methods intentionally
//! left out, and why.
#![cfg(test)]
#![allow(clippy::too_many_lines)]

use crate::*;

fn fixture_store() -> Store {
    let mut store = Store::new(Document {
        id: id(),
        name: "Generated bindings fixture".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Fixture".into(),
            grid: vec![
                vec!["Num".into(), "Words".into(), "Flag".into(), "When".into()],
                vec![
                    "4".into(),
                    "Hello World".into(),
                    "True".into(),
                    "2024-01-15".into(),
                ],
                vec![
                    "9".into(),
                    "polars rocks".into(),
                    "False".into(),
                    "2024-03-20".into(),
                ],
                vec![
                    "16".into(),
                    "  trim me  ".into(),
                    "True".into(),
                    "2024-06-01".into(),
                ],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
}

fn frame_id(store: &Store) -> Id {
    store
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap()
}

fn assert_formula_executes(store: &mut Store, frame_id: &Id, name: &str, formula: &str) {
    let result = store.apply(Operation::AddComputedColumn {
        frame_id: frame_id.clone(),
        name: name.into(),
        formula: formula.into(),
        after_column_id: None,
    });
    assert!(
        result.is_ok(),
        "formula `{formula}` (as column `{name}`) failed: {:?}",
        result.err()
    );
}

fn assert_ordered_formula_executes(name: &str, formula: &str) {
    let mut store = fixture_store();
    let frame_id = frame_id(&store);
    let num_id = store.document.frame(&frame_id).unwrap().columns[0]
        .id
        .clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::Sort {
                keys: vec![SortInput {
                    column_id: num_id,
                    descending: false,
                }],
            }],
        })
        .unwrap();
    assert_formula_executes(&mut store, &frame_id, name, formula);
}

#[test]
fn generated_dt_bindings_execute_without_error() {
    let mut store = fixture_store();
    let frame_id = frame_id(&store);
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_dt_0_century",
        "`When`.dt.century()",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_dt_1_day", "`When`.dt.day()");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_dt_2_millennium",
        "`When`.dt.millennium()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_dt_3_replace",
        "`When`.dt.replace(1, 1, 1, 1, 1, 1, 1, \"raise\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_dt_4_round",
        "`When`.dt.round(\"1mo\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_dt_5_truncate",
        "`When`.dt.truncate(\"1mo\")",
    );
}

#[test]
fn generated_expr_bindings_execute_without_error() {
    let mut store = fixture_store();
    let frame_id = frame_id(&store);
    assert_formula_executes(&mut store, &frame_id, "gen_expr_0_all", "`Num`.all(True)");
    assert_formula_executes(&mut store, &frame_id, "gen_expr_1_any", "`Num`.any(True)");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_2_approx_n_unique",
        "`Num`.approx_n_unique()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_3_arg_max",
        "`Num`.arg_max()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_4_arg_min",
        "`Num`.arg_min()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_5_arg_sort",
        "`Num`.arg_sort(True, True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_6_bitwise_and",
        "`Flag`.bitwise_and()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_7_bitwise_count_ones",
        "`Flag`.bitwise_count_ones()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_8_bitwise_count_zeros",
        "`Flag`.bitwise_count_zeros()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_9_bitwise_leading_ones",
        "`Flag`.bitwise_leading_ones()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_10_bitwise_leading_zeros",
        "`Flag`.bitwise_leading_zeros()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_11_bitwise_or",
        "`Flag`.bitwise_or()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_12_bitwise_trailing_ones",
        "`Flag`.bitwise_trailing_ones()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_13_bitwise_trailing_zeros",
        "`Flag`.bitwise_trailing_zeros()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_14_bitwise_xor",
        "`Flag`.bitwise_xor()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_15_bottom_k",
        "`Num`.bottom_k(3)",
    );
    assert_ordered_formula_executes("gen_expr_16_cum_count", "`Num`.cum_count(True)");
    assert_ordered_formula_executes("gen_expr_17_cum_max", "`Num`.cum_max(True)");
    assert_ordered_formula_executes("gen_expr_18_cum_min", "`Num`.cum_min(True)");
    assert_ordered_formula_executes("gen_expr_19_cum_prod", "`Num`.cum_prod(True)");
    assert_ordered_formula_executes("gen_expr_20_cum_sum", "`Num`.cum_sum(True)");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_21_entropy",
        "`Num`.entropy(1.5, True)",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_expr_22_first", "`Num`.first()");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_23_first_non_null",
        "`Num`.first_non_null()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_24_has_nulls",
        "`Num`.has_nulls()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_25_hash",
        "`Num`.hash(1, 1, 1, 1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_26_interpolate_by",
        "`Num`.interpolate_by(`Num`)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_27_is_duplicated",
        "`Num`.is_duplicated()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_28_is_empty",
        "`Num`.is_empty(True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_29_is_finite",
        "`Num`.is_finite()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_30_is_first_distinct",
        "`Num`.is_first_distinct()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_31_is_infinite",
        "`Num`.is_infinite()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_32_is_last_distinct",
        "`Num`.is_last_distinct()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_33_is_nan",
        "`Num`.is_nan()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_34_is_not_nan",
        "`Num`.is_not_nan()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_35_is_sorted",
        "`Num`.is_sorted(True, True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_36_is_unique",
        "`Num`.is_unique()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_37_kurtosis",
        "`Num`.kurtosis(True, True)",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_expr_38_last", "`Num`.last()");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_39_last_non_null",
        "`Num`.last_non_null()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_40_lower_bound",
        "`Num`.lower_bound()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_41_max_by",
        "`Num`.max_by(`Num`)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_42_median",
        "`Num`.median()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_43_min_by",
        "`Num`.min_by(`Num`)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_44_mode",
        "`Num`.mode(True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_45_n_unique",
        "`Num`.n_unique()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_46_nan_max",
        "`Num`.nan_max()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_47_nan_min",
        "`Num`.nan_min()",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_expr_48_not", "`Flag`.not()");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_49_pct_change",
        "`Num`.pct_change(2)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_50_peak_max",
        "`Num`.peak_max()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_51_peak_min",
        "`Num`.peak_min()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_52_product",
        "`Num`.product()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_53_rechunk",
        "`Num`.rechunk()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_54_reverse",
        "`Num`.reverse()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_55_rle_id",
        "`Num`.rle_id()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_56_shuffle",
        "`Num`.shuffle(7)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_57_skew",
        "`Num`.skew(True)",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_expr_58_std", "`Num`.std(1)");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_59_to_physical",
        "`Num`.to_physical()",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_expr_60_top_k", "`Num`.top_k(3)");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_61_true_div",
        "`Num`.true_div(2)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_62_unique_counts",
        "`Num`.unique_counts()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_expr_63_upper_bound",
        "`Num`.upper_bound()",
    );
    assert_formula_executes(&mut store, &frame_id, "gen_expr_64_var", "`Num`.var(1)");
}

#[test]
fn generated_root_fn_bindings_execute_without_error() {
    let mut store = fixture_store();
    let frame_id = frame_id(&store);
    assert_formula_executes(&mut store, &frame_id, "gen_root_fn_0_cov", "cov(2, 2, 1)");
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_root_fn_1_pearson_corr",
        "pearson_corr(2, 2)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_root_fn_2_spearman_rank_corr",
        "spearman_rank_corr(2, 2, True)",
    );
}

#[test]
fn generated_str_bindings_execute_without_error() {
    let mut store = fixture_store();
    let frame_id = frame_id(&store);
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_0_base64_encode",
        "`Words`.str.base64_encode()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_1_contains_any",
        "`Words`.str.contains_any(\"a\", True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_2_contains_literal",
        "`Words`.str.contains_literal(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_3_count_matches",
        "`Words`.str.count_matches(\"a\", True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_4_ends_with",
        "`Words`.str.ends_with(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_5_escape_regex",
        "`Words`.str.escape_regex()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_6_extract",
        "`Words`.str.extract(\"a\", 1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_7_find",
        "`Words`.str.find(\"a\", False)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_8_find_literal",
        "`Words`.str.find_literal(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_9_head",
        "`Words`.str.head(1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_10_hex_encode",
        "`Words`.str.hex_encode()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_11_json_path_match",
        "`Words`.str.json_path_match(\"$.a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_12_len_bytes",
        "`Words`.str.len_bytes()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_13_len_chars",
        "`Words`.str.len_chars()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_14_replace",
        "`Words`.str.replace(\"a\", \"a\", True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_15_replace_all",
        "`Words`.str.replace_all(\"a\", \"a\", True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_16_replace_many",
        "`Words`.str.replace_many(\"a\", \"a\", True, True)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_17_replace_n",
        "`Words`.str.replace_n(\"a\", \"a\", True, 1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_18_reverse",
        "`Words`.str.reverse()",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_19_slice",
        "`Words`.str.slice(\"a\", 1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_20_starts_with",
        "`Words`.str.starts_with(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_21_strip_chars",
        "`Words`.str.strip_chars(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_22_strip_chars_end",
        "`Words`.str.strip_chars_end(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_23_strip_chars_start",
        "`Words`.str.strip_chars_start(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_24_strip_prefix",
        "`Words`.str.strip_prefix(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_25_strip_suffix",
        "`Words`.str.strip_suffix(\"a\")",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_26_tail",
        "`Words`.str.tail(1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_27_to_decimal",
        "`Words`.str.to_decimal(1)",
    );
    assert_formula_executes(
        &mut store,
        &frame_id,
        "gen_str_28_zfill",
        "`Words`.str.zfill(1)",
    );
}
