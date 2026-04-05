use alpaca_trade::clock::Clock;

#[test]
fn clock_model_deserializes_official_shape() {
    let json = r#"
    {
      "timestamp": "2024-04-05T13:30:00Z",
      "is_open": true,
      "next_open": "2024-04-08T13:30:00Z",
      "next_close": "2024-04-05T20:00:00Z"
    }
    "#;

    let clock: Clock = serde_json::from_str(json).expect("json should deserialize");
    assert_eq!(clock.timestamp, "2024-04-05T13:30:00Z");
    assert!(clock.is_open);
    assert_eq!(clock.next_open, "2024-04-08T13:30:00Z");
    assert_eq!(clock.next_close, "2024-04-05T20:00:00Z");
}

#[test]
fn clock_model_rejects_missing_required_timestamp() {
    let json = r#"
    {
      "is_open": false,
      "next_open": "2024-04-08T13:30:00Z",
      "next_close": "2024-04-05T20:00:00Z"
    }
    "#;

    let error = serde_json::from_str::<Clock>(json).expect_err("missing timestamp must fail");
    assert!(error.to_string().contains("timestamp"));
}
