use super::OperationId;
#[test]
fn base36_boundaries_and_leading_zeroes_round_trip() {
    for (value, text) in [
        (0, "oppc-00000000"),
        (9, "oppc-00000009"),
        (10, "oppc-0000000a"),
        (35, "oppc-0000000z"),
        (36, "oppc-00000010"),
        (OperationId::LIMIT - 1, "oppc-zzzzzzzz"),
    ] {
        let id = OperationId::from_stored(value).unwrap();
        assert_eq!(id.to_string(), text);
        assert_eq!(OperationId::parse(text).unwrap(), id);
        assert_eq!(id.value(), value);
    }
}
#[test]
fn parsing_requires_the_complete_lowercase_ascii_format() {
    for invalid in [
        "",
        "00000000",
        "oppc-",
        "oppc-0000000",
        "oppc-000000000",
        "oppc_00000000",
        "oppp-00000000",
        "OPPC-00000000",
        "oppc-0000000A",
        "oppc-0000000_",
        "oppc-0000000-",
        "oppc-+0000000",
        "oppc-000000é",
        "oppc-００００",
        " oppc-00000000",
        "oppc-00000000 ",
        "oppc-00000000\n",
        "oppc-0000000\0",
        "019d0000-0000-7000-8000-000000000103",
    ] {
        let error = OperationId::parse(invalid).unwrap_err();
        assert!(error.to_string().contains("8 lowercase Base36 digits"));
    }
}
#[test]
fn stored_numbers_outside_the_id_space_are_rejected() {
    for invalid in [i64::MIN, -1, OperationId::LIMIT, i64::MAX] {
        let error = OperationId::from_stored(invalid).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid operation ID in operation history")
        );
    }
}
#[test]
fn generated_ids_stay_within_the_fixed_width_space() {
    for _ in 0_u16..4096 {
        let id = OperationId::random();
        let text = id.to_string();
        assert_eq!(text.len(), 13);
        assert_eq!(OperationId::parse(&text).unwrap(), id);
        assert_eq!(OperationId::from_stored(id.value()).unwrap(), id);
    }
}
