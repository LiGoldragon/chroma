//! Round-trip tests for the [`chroma::Response`] enum.

use chroma::Response;

#[test]
fn accepted_response_uses_human_readable_nota_word() {
    let response = Response::Accepted;

    let rendered = response.to_nota().expect("nota encode");
    assert_eq!(rendered, "Accepted");
    assert_eq!(Response::from_nota(&rendered).expect("nota decode"), response);
}

#[test]
fn accepted_response_round_trips_through_rkyv_wire_format() {
    let response = Response::Accepted;
    let bytes = response.archive().expect("archive");

    assert_eq!(Response::from_archive(&bytes).expect("from_archive"), response);
}

#[test]
fn solar_clock_response_round_trips_without_a_coordinate() {
    let response = Response::SolarClock { utc_offset_seconds: -854, valid_until_unix_seconds: 1_736_208_000 };
    let rendered = response.to_nota().expect("nota encode");

    assert_eq!(rendered, "(SolarClock -854 1736208000)");
    assert_eq!(Response::from_nota(&rendered).expect("nota decode"), response);
    assert_eq!(Response::from_archive(&response.archive().expect("archive")).expect("decode"), response);
}

#[test]
fn unavailable_solar_clock_round_trips_explicitly() {
    let response = Response::SolarClockUnavailable;
    assert_eq!(response.to_nota().expect("nota encode"), "SolarClockUnavailable");
    assert_eq!(Response::from_nota("SolarClockUnavailable").expect("nota decode"), response);
}

#[test]
fn error_messages_with_apostrophes_do_not_require_quote_delimiters() {
    let response = Response::Error { message: "theme's palette is missing".into() };

    let rendered = response.to_nota().expect("nota encode");
    assert_eq!(rendered, "(Error [theme's palette is missing])");
    assert_eq!(Response::from_nota(&rendered).expect("nota decode"), response);
}
