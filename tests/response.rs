//! Generated Datom reply boundary and runtime-only rkyv frame tests.

use chroma::Response;
use datom_codec::Textualizable;

#[test]
fn accepted_reply_is_generated_datom() {
    let reply = chroma::generated::Reply::try_from(Response::Accepted).expect("render reply");
    assert_eq!(reply.textualize(), "Accepted");
}

#[test]
fn solar_clock_reply_is_named_and_positional_in_its_generated_anatomy() {
    let response =
        Response::SolarClock { utc_offset_seconds: -854, equation_of_time_valid_until_unix_seconds: 1_736_208_000 };
    let reply = chroma::generated::Reply::try_from(response.clone()).expect("render reply");
    assert_eq!(reply.textualize(), "SolarClock.{ -854 1736208000 }");
    assert_eq!(Response::from_archive(&response.archive().expect("archive")).expect("decode"), response);
}

#[test]
fn unavailable_solar_clock_is_explicit() {
    let reply = chroma::generated::Reply::try_from(Response::SolarClockUnavailable).expect("render reply");
    assert_eq!(reply.textualize(), "SolarClockUnavailable");
}

#[test]
fn reply_strings_use_datom_quotes() {
    let reply = chroma::generated::Reply::try_from(Response::Error { message: "theme's palette is missing".into() })
        .expect("render reply");
    assert_eq!(reply.textualize(), "Error.{ “theme's palette is missing” }");
}
