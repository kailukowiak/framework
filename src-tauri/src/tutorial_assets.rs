/// A source workbook travels in the desktop bundle, so an installed build
/// can make the tutorial folder without needing this checkout or a network
/// connection. Start and answer-key copies use different parent directories:
/// their canonical files intentionally share document IDs, and collaboration
/// sidecars are keyed by that ID beneath a document's parent directory.
pub(super) struct BundledTutorial {
    pub(super) lesson: &'static str,
    pub(super) kind: &'static str,
    pub(super) relative_path: &'static str,
    pub(super) contents: &'static [u8],
    pub(super) assets: &'static [BundledTutorialAsset],
}

pub(super) struct BundledTutorialAsset {
    pub(super) relative_path: &'static str,
    pub(super) contents: &'static [u8],
}

const EXCEL_START_ASSETS: &[BundledTutorialAsset] = &[
    BundledTutorialAsset {
        relative_path: "simple-customers.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/simple-customers.xlsx"),
    },
    BundledTutorialAsset {
        relative_path: "multi-table-operations.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/multi-table-operations.xlsx"),
    },
];

const EXCEL_FINISHED_ASSETS: &[BundledTutorialAsset] = &[
    BundledTutorialAsset {
        relative_path: "simple-customers.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/simple-customers.xlsx"),
    },
    BundledTutorialAsset {
        relative_path: "multi-table-operations.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/multi-table-operations.xlsx"),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/22141794ab301df27ad6926c4a695aaf4c44a69ea570924cc20706e6753d26a4.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/22141794ab301df27ad6926c4a695aaf4c44a69ea570924cc20706e6753d26a4.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/31743e7aabd9104b7499a9bb55b533db5313c98f6dcca7f4d220b2f9d2620216.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/31743e7aabd9104b7499a9bb55b533db5313c98f6dcca7f4d220b2f9d2620216.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/8dc284978b1a45639111232b51562167bf56f6051329263ef5c0119ec0766411.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/8dc284978b1a45639111232b51562167bf56f6051329263ef5c0119ec0766411.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/de982e3a709421e05a6804a2a09cda224d0194b27cad4d4455129c120e702061.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/de982e3a709421e05a6804a2a09cda224d0194b27cad4d4455129c120e702061.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/e1527918fc567a68ca7d844f984ad658b63d0e05155e5776675222c0a36284ce.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/e1527918fc567a68ca7d844f984ad658b63d0e05155e5776675222c0a36284ce.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/e1cc295c5c0f9b2c218dca3a3e08680ab000a1d8c3f84ddc92cadd334f2e51b8.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/e1cc295c5c0f9b2c218dca3a3e08680ab000a1d8c3f84ddc92cadd334f2e51b8.parquet"
        ),
    },
];

pub(super) const BUNDLED_TUTORIALS: &[BundledTutorial] = &[
    // The order here is the order the library lists them in, and the tour is
    // lesson zero: it is the one a person who has never opened FrameWork
    // should reach first.
    BundledTutorial {
        lesson: "The FrameWork tour",
        kind: "Start",
        relative_path: "The FrameWork tour/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/grand-tour/grand-tour-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "The FrameWork tour",
        kind: "Answer key",
        relative_path: "The FrameWork tour/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/grand-tour/grand-tour-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Your first FrameWork workbook",
        kind: "Start",
        relative_path: "Your first FrameWork workbook/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/first-workbook/first-workbook-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Your first FrameWork workbook",
        kind: "Answer key",
        relative_path: "Your first FrameWork workbook/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/first-workbook/first-workbook-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Importing an Excel workbook",
        kind: "Start",
        relative_path: "Importing an Excel workbook/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/excel-import/excel-import-start.fw"),
        assets: EXCEL_START_ASSETS,
    },
    BundledTutorial {
        lesson: "Importing an Excel workbook",
        kind: "Answer key",
        relative_path: "Importing an Excel workbook/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/excel-import/excel-import-finished.fw"),
        assets: EXCEL_FINISHED_ASSETS,
    },
    BundledTutorial {
        lesson: "Month-over-month formulas by pointing",
        kind: "Start",
        relative_path: "Month-over-month formulas by pointing/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/formula-clicks/formula-clicks-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Month-over-month formulas by pointing",
        kind: "Answer key",
        relative_path: "Month-over-month formulas by pointing/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/formula-clicks/formula-clicks-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Month-end close",
        kind: "Start",
        relative_path: "Month-end close/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/month-end-close/month-end-close-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Month-end close",
        kind: "Answer key",
        relative_path: "Month-end close/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/month-end-close/month-end-close-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Vectors, dates, and visual joins",
        kind: "Start",
        relative_path: "Vectors, dates, and visual joins/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/vectors-and-joins/vectors-and-joins-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Vectors, dates, and visual joins",
        kind: "Answer key",
        relative_path: "Vectors, dates, and visual joins/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/vectors-and-joins/vectors-and-joins-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Dictionaries and value mapping",
        kind: "Start",
        relative_path: "Dictionaries and value mapping/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/dictionaries/dictionaries-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Dictionaries and value mapping",
        kind: "Answer key",
        relative_path: "Dictionaries and value mapping/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/dictionaries/dictionaries-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Robust standard errors with diabetes progression",
        kind: "Start",
        relative_path: "Robust standard errors with diabetes progression/Start/Workbook.fw",
        contents: include_bytes!(
            "../../tutorials/robust-linear-regression/robust-linear-regression-start.fw"
        ),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Robust standard errors with diabetes progression",
        kind: "Answer key",
        relative_path: "Robust standard errors with diabetes progression/Answer key/Workbook.fw",
        contents: include_bytes!(
            "../../tutorials/robust-linear-regression/robust-linear-regression-finished.fw"
        ),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Classify Iris species with XGBoost",
        kind: "Start",
        relative_path: "Classify Iris species with XGBoost/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/xgboost/xgboost-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Classify Iris species with XGBoost",
        kind: "Answer key",
        relative_path: "Classify Iris species with XGBoost/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/xgboost/xgboost-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Price a deal",
        kind: "Start",
        relative_path: "Price a deal/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/price-a-deal/price-a-deal-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Price a deal",
        kind: "Answer key",
        relative_path: "Price a deal/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/price-a-deal/price-a-deal-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Driver-based forecast",
        kind: "Start",
        relative_path: "Driver-based forecast/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/driver-forecast/driver-forecast-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Driver-based forecast",
        kind: "Answer key",
        relative_path: "Driver-based forecast/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/driver-forecast/driver-forecast-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Scenarios, sensitivity and goal seek",
        kind: "Start",
        relative_path: "Scenarios, sensitivity and goal seek/Start/Workbook.fw",
        contents: include_bytes!(
            "../../tutorials/scenarios-and-sensitivity/scenarios-and-sensitivity-start.fw"
        ),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Scenarios, sensitivity and goal seek",
        kind: "Answer key",
        relative_path: "Scenarios, sensitivity and goal seek/Answer key/Workbook.fw",
        contents: include_bytes!(
            "../../tutorials/scenarios-and-sensitivity/scenarios-and-sensitivity-finished.fw"
        ),
        assets: &[],
    },
];
