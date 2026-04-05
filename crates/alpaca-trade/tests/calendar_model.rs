use alpaca_trade::calendar::Calendar;

#[test]
fn calendar_model_deserializes_official_array_shape() {
    let json = r#"
    [
      {
        "close": "16:00",
        "date": "2026-04-01",
        "open": "09:30",
        "session_close": "2000",
        "session_open": "0400",
        "settlement_date": "2026-04-02"
      }
    ]
    "#;

    let calendar: Vec<Calendar> = serde_json::from_str(json).expect("json should deserialize");
    assert_eq!(calendar.len(), 1);
    assert_eq!(calendar[0].close, "16:00");
    assert_eq!(calendar[0].date, "2026-04-01");
    assert_eq!(calendar[0].open, "09:30");
    assert_eq!(calendar[0].session_close, "2000");
    assert_eq!(calendar[0].session_open, "0400");
    assert_eq!(calendar[0].settlement_date, "2026-04-02");
}

#[test]
fn calendar_model_rejects_missing_required_date() {
    let json = r#"
    [
      {
        "close": "16:00",
        "open": "09:30",
        "session_close": "2000",
        "session_open": "0400",
        "settlement_date": "2026-04-02"
      }
    ]
    "#;

    let error = serde_json::from_str::<Vec<Calendar>>(json).expect_err("missing date must fail");
    assert!(error.to_string().contains("date"));
}
