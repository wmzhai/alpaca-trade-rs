use alpaca_trade::Decimal;
use alpaca_trade::options_contracts::{
    ContractStatus, ContractStyle, ContractType, DeliverableSettlementMethod,
    DeliverableSettlementType, DeliverableType, ListResponse, OptionContract,
};
use std::str::FromStr;

#[test]
fn list_response_deserializes_official_shape_with_deliverables() {
    let json = r#"
    {
      "option_contracts": [
        {
          "id": "98359ef7-5124-49f3-85ea-5cf02df6defa",
          "symbol": "AAPL250620C00100000",
          "name": "AAPL Jun 20 2025 100 Call",
          "status": "active",
          "tradable": true,
          "expiration_date": "2025-06-20",
          "root_symbol": "AAPL",
          "underlying_symbol": "AAPL",
          "underlying_asset_id": "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
          "type": "call",
          "style": "american",
          "strike_price": "100",
          "multiplier": "100",
          "size": "100",
          "open_interest": "237",
          "open_interest_date": "2023-12-11",
          "close_price": "148.38",
          "close_price_date": "2023-12-11",
          "deliverables": [
            {
              "type": "equity",
              "symbol": "AAPL",
              "asset_id": "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
              "amount": "100",
              "allocation_percentage": "100",
              "settlement_type": "T+2",
              "settlement_method": "CCC",
              "delayed_settlement": false
            }
          ]
        }
      ],
      "next_page_token": "MTAwMA=="
    }
    "#;

    let response: ListResponse = serde_json::from_str(json).expect("json should deserialize");

    assert_eq!(response.option_contracts.len(), 1);
    let contract = &response.option_contracts[0];
    assert_eq!(contract.status, ContractStatus::Active);
    assert_eq!(contract.r#type, ContractType::Call);
    assert_eq!(contract.style, ContractStyle::American);
    assert_eq!(contract.strike_price, Decimal::from_str("100").unwrap());
    assert_eq!(contract.multiplier, Decimal::from_str("100").unwrap());
    assert_eq!(contract.size, Decimal::from_str("100").unwrap());
    assert_eq!(
        contract.open_interest,
        Some(Decimal::from_str("237").unwrap())
    );
    assert_eq!(
        contract.close_price,
        Some(Decimal::from_str("148.38").unwrap())
    );
    assert_eq!(response.next_page_token.as_deref(), Some("MTAwMA=="));
    let deliverable = contract.deliverables.as_ref().unwrap().first().unwrap();
    assert_eq!(deliverable.r#type, DeliverableType::Equity);
    assert_eq!(deliverable.amount, Some(Decimal::from_str("100").unwrap()));
    assert_eq!(
        deliverable.allocation_percentage,
        Decimal::from_str("100").unwrap()
    );
    assert_eq!(
        deliverable.settlement_type,
        DeliverableSettlementType::TPlus2
    );
    assert_eq!(
        deliverable.settlement_method,
        DeliverableSettlementMethod::Ccc
    );
}

#[test]
fn option_contract_accepts_missing_optional_fields() {
    let json = r#"
    {
      "id": "98359ef7-5124-49f3-85ea-5cf02df6defa",
      "symbol": "AAPL250620C00100000",
      "name": "AAPL Jun 20 2025 100 Call",
      "status": "inactive",
      "tradable": false,
      "expiration_date": "2025-06-20",
      "underlying_symbol": "AAPL",
      "underlying_asset_id": "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
      "type": "put",
      "style": "european",
      "strike_price": 100,
      "multiplier": 100,
      "size": 100
    }
    "#;

    let contract: OptionContract = serde_json::from_str(json).expect("json should deserialize");

    assert_eq!(contract.root_symbol, None);
    assert_eq!(contract.open_interest, None);
    assert_eq!(contract.open_interest_date, None);
    assert_eq!(contract.close_price, None);
    assert_eq!(contract.close_price_date, None);
    assert_eq!(contract.deliverables, None);
    assert_eq!(contract.status, ContractStatus::Inactive);
    assert_eq!(contract.r#type, ContractType::Put);
    assert_eq!(contract.style, ContractStyle::European);
}

#[test]
fn option_contract_rejects_missing_required_symbol() {
    let json = r#"
    {
      "id": "98359ef7-5124-49f3-85ea-5cf02df6defa",
      "name": "AAPL Jun 20 2025 100 Call",
      "status": "active",
      "tradable": true,
      "expiration_date": "2025-06-20",
      "underlying_symbol": "AAPL",
      "underlying_asset_id": "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
      "type": "call",
      "style": "american",
      "strike_price": "100",
      "multiplier": "100",
      "size": "100"
    }
    "#;

    let error = serde_json::from_str::<OptionContract>(json).expect_err("missing symbol must fail");
    assert!(error.to_string().contains("missing field `symbol`"));
}
