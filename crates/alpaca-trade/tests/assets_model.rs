use alpaca_trade::{Decimal, assets::Asset};
use std::str::FromStr;

#[test]
fn asset_model_deserializes_official_list_shape() {
    let json = r#"
    [
      {
        "id": "904837e3-3b76-47ec-b432-046db621571b",
        "class": "us_equity",
        "exchange": "NASDAQ",
        "symbol": "AAPL",
        "name": "Apple Inc. Common Stock",
        "status": "active",
        "tradable": true,
        "marginable": true,
        "shortable": true,
        "easy_to_borrow": true,
        "fractionable": true,
        "cusip": "037833100",
        "maintenance_margin_requirement": 30.0,
        "margin_requirement_long": "30",
        "margin_requirement_short": "100",
        "attributes": ["fractional_eh_enabled", "has_options"]
      }
    ]
    "#;

    let assets: Vec<Asset> = serde_json::from_str(json).expect("json should deserialize");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id, "904837e3-3b76-47ec-b432-046db621571b");
    assert_eq!(assets[0].class, "us_equity");
    assert_eq!(assets[0].exchange, "NASDAQ");
    assert_eq!(assets[0].symbol, "AAPL");
    assert_eq!(assets[0].name, "Apple Inc. Common Stock");
    assert_eq!(assets[0].status, "active");
    assert!(assets[0].tradable);
    assert!(assets[0].marginable);
    assert!(assets[0].shortable);
    assert!(assets[0].easy_to_borrow);
    assert!(assets[0].fractionable);
    assert_eq!(assets[0].cusip.as_deref(), Some("037833100"));
    assert_eq!(
        assets[0].maintenance_margin_requirement,
        Some(Decimal::from_str("30.0").expect("decimal should parse"))
    );
    assert_eq!(
        assets[0].margin_requirement_long,
        Some(Decimal::from_str("30").expect("decimal should parse"))
    );
    assert_eq!(
        assets[0].margin_requirement_short,
        Some(Decimal::from_str("100").expect("decimal should parse"))
    );
    let expected_attributes = vec!["fractional_eh_enabled".to_owned(), "has_options".to_owned()];
    assert_eq!(
        assets[0].attributes.as_ref().map(Vec::as_slice),
        Some(expected_attributes.as_slice())
    );
}

#[test]
fn asset_model_deserializes_single_shape_with_optional_fields_missing() {
    let json = r#"
    {
      "id": "904837e3-3b76-47ec-b432-046db621571b",
      "class": "us_equity",
      "exchange": "NASDAQ",
      "symbol": "AAPL",
      "name": "Apple Inc. Common Stock",
      "status": "active",
      "tradable": true,
      "marginable": true,
      "shortable": true,
      "easy_to_borrow": true,
      "fractionable": true
    }
    "#;

    let asset: Asset = serde_json::from_str(json).expect("json should deserialize");
    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(asset.cusip, None);
    assert_eq!(asset.maintenance_margin_requirement, None);
    assert_eq!(asset.margin_requirement_long, None);
    assert_eq!(asset.margin_requirement_short, None);
    assert_eq!(asset.attributes, None);
}

#[test]
fn asset_model_accepts_supported_string_and_number_shapes() {
    let json = r#"
    {
      "id": "904837e3-3b76-47ec-b432-046db621571b",
      "class": "us_equity",
      "exchange": "NASDAQ",
      "symbol": "AAPL",
      "name": "Apple Inc. Common Stock",
      "status": "active",
      "tradable": true,
      "marginable": true,
      "shortable": true,
      "easy_to_borrow": true,
      "fractionable": true,
      "maintenance_margin_requirement": "30.0",
      "margin_requirement_long": 30,
      "margin_requirement_short": 100
    }
    "#;

    let asset: Asset = serde_json::from_str(json).expect("json should deserialize");

    assert_eq!(
        asset.maintenance_margin_requirement,
        Some(Decimal::from_str("30.0").expect("decimal should parse"))
    );
    assert_eq!(
        asset.margin_requirement_long,
        Some(Decimal::from_str("30").expect("decimal should parse"))
    );
    assert_eq!(
        asset.margin_requirement_short,
        Some(Decimal::from_str("100").expect("decimal should parse"))
    );
}

#[test]
fn asset_model_serializes_decimal_fields_with_official_wire_contracts() {
    let json = r#"
    {
      "id": "904837e3-3b76-47ec-b432-046db621571b",
      "class": "us_equity",
      "exchange": "NASDAQ",
      "symbol": "AAPL",
      "name": "Apple Inc. Common Stock",
      "status": "active",
      "tradable": true,
      "marginable": true,
      "shortable": true,
      "easy_to_borrow": true,
      "fractionable": true,
      "maintenance_margin_requirement": "30.0",
      "margin_requirement_long": 30,
      "margin_requirement_short": 100
    }
    "#;

    let asset: Asset = serde_json::from_str(json).expect("json should deserialize");
    let value = serde_json::to_value(&asset).expect("asset should serialize");

    assert!(value["maintenance_margin_requirement"].is_number());
    assert_eq!(value["maintenance_margin_requirement"].to_string(), "30.0");
    assert_eq!(value["margin_requirement_long"], serde_json::json!("30"));
    assert_eq!(value["margin_requirement_short"], serde_json::json!("100"));
}

#[test]
fn asset_model_rejects_missing_required_symbol() {
    let json = r#"
    {
      "id": "904837e3-3b76-47ec-b432-046db621571b",
      "class": "us_equity",
      "exchange": "NASDAQ",
      "name": "Apple Inc. Common Stock",
      "status": "active",
      "tradable": true,
      "marginable": true,
      "shortable": true,
      "easy_to_borrow": true,
      "fractionable": true
    }
    "#;

    let error = serde_json::from_str::<Asset>(json).expect_err("missing symbol must fail");
    assert!(error.to_string().contains("missing field `symbol`"));
}
