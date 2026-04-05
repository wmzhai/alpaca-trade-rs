use alpaca_trade::assets::Asset;

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
    assert_eq!(assets[0].maintenance_margin_requirement, Some(30.0));
    assert_eq!(assets[0].margin_requirement_long.as_deref(), Some("30"));
    assert_eq!(assets[0].margin_requirement_short.as_deref(), Some("100"));
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
