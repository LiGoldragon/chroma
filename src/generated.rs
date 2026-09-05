#![allow(dead_code)]
#![allow(clippy::redundant_closure)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    SetTheme(RequestSetTheme),
    GetTheme,
    SetWarmth(RequestSetWarmth),
    SetWarmthKelvin(RequestSetWarmthKelvin),
    StartWarmthRamp(RequestStartWarmthRamp),
    StartWarmthRampKelvin(RequestStartWarmthRampKelvin),
    InterruptWarmth,
    GetWarmth,
    SetBrightness(RequestSetBrightness),
    SetBrightnessPercent(RequestSetBrightnessPercent),
    StartBrightnessRamp(RequestStartBrightnessRamp),
    StartBrightnessRampPercent(RequestStartBrightnessRampPercent),
    InterruptBrightness,
    GetBrightness,
    GetState,
    GetSolarClock,
}
impl datom_codec::Datomic for Request {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "SetTheme" => std::result::Result::Ok(Self::SetTheme(datom_codec::Carrying::body(v)?)),
            "GetTheme" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::GetTheme)
            }
            "SetWarmth" => std::result::Result::Ok(Self::SetWarmth(datom_codec::Carrying::body(v)?)),
            "SetWarmthKelvin" => std::result::Result::Ok(Self::SetWarmthKelvin(datom_codec::Carrying::body(v)?)),
            "StartWarmthRamp" => std::result::Result::Ok(Self::StartWarmthRamp(datom_codec::Carrying::body(v)?)),
            "StartWarmthRampKelvin" => {
                std::result::Result::Ok(Self::StartWarmthRampKelvin(datom_codec::Carrying::body(v)?))
            }
            "InterruptWarmth" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::InterruptWarmth)
            }
            "GetWarmth" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::GetWarmth)
            }
            "SetBrightness" => std::result::Result::Ok(Self::SetBrightness(datom_codec::Carrying::body(v)?)),
            "SetBrightnessPercent" => {
                std::result::Result::Ok(Self::SetBrightnessPercent(datom_codec::Carrying::body(v)?))
            }
            "StartBrightnessRamp" => {
                std::result::Result::Ok(Self::StartBrightnessRamp(datom_codec::Carrying::body(v)?))
            }
            "StartBrightnessRampPercent" => {
                std::result::Result::Ok(Self::StartBrightnessRampPercent(datom_codec::Carrying::body(v)?))
            }
            "InterruptBrightness" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::InterruptBrightness)
            }
            "GetBrightness" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::GetBrightness)
            }
            "GetState" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::GetState)
            }
            "GetSolarClock" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::GetSolarClock)
            }
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for Request {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::SetTheme(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("SetTheme").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::GetTheme => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("GetTheme").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::SetWarmth(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("SetWarmth").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::SetWarmthKelvin(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("SetWarmthKelvin").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::StartWarmthRamp(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("StartWarmthRamp").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::StartWarmthRampKelvin(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("StartWarmthRampKelvin").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::InterruptWarmth => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(
                        protos::Word::try_from("InterruptWarmth").expect("static variant"),
                    )
                    .expect("stable variant"),
                ),
                Self::GetWarmth => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("GetWarmth").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::SetBrightness(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("SetBrightness").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::SetBrightnessPercent(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("SetBrightnessPercent").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::StartBrightnessRamp(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("StartBrightnessRamp").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::StartBrightnessRampPercent(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("StartBrightnessRampPercent").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::InterruptBrightness => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(
                        protos::Word::try_from("InterruptBrightness").expect("static variant"),
                    )
                    .expect("stable variant"),
                ),
                Self::GetBrightness => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("GetBrightness").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::GetState => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("GetState").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::GetSolarClock => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("GetSolarClock").expect("static variant"))
                        .expect("stable variant"),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestSetTheme(pub ThemeMode);
impl datom_codec::Datomic for RequestSetTheme {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: ThemeMode = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestSetTheme {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestSetWarmth(pub WarmthLevel);
impl datom_codec::Datomic for RequestSetWarmth {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: WarmthLevel = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestSetWarmth {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestSetWarmthKelvin(pub protos::Integer);
impl datom_codec::Datomic for RequestSetWarmthKelvin {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestSetWarmthKelvin {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestStartWarmthRamp(pub WarmthLevel, pub RampDuration);
impl datom_codec::Datomic for RequestStartWarmthRamp {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: WarmthLevel = datom_codec::Positional::position(&mut p)?;
        let p1: RampDuration = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestStartWarmthRamp {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestStartWarmthRampKelvin(pub protos::Integer, pub RampDuration);
impl datom_codec::Datomic for RequestStartWarmthRampKelvin {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        let p1: RampDuration = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestStartWarmthRampKelvin {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestSetBrightness(pub BrightnessLevel);
impl datom_codec::Datomic for RequestSetBrightness {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: BrightnessLevel = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestSetBrightness {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestSetBrightnessPercent(pub protos::Integer);
impl datom_codec::Datomic for RequestSetBrightnessPercent {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestSetBrightnessPercent {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestStartBrightnessRamp(pub BrightnessLevel, pub RampDuration);
impl datom_codec::Datomic for RequestStartBrightnessRamp {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: BrightnessLevel = datom_codec::Positional::position(&mut p)?;
        let p1: RampDuration = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestStartBrightnessRamp {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestStartBrightnessRampPercent(pub protos::Integer, pub RampDuration);
impl datom_codec::Datomic for RequestStartBrightnessRampPercent {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        let p1: RampDuration = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RequestStartBrightnessRampPercent {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Accepted,
    Theme(ReplyTheme),
    Warmth(ReplyWarmth),
    Brightness(ReplyBrightness),
    State(ReplyState),
    SolarClock(ReplySolarClock),
    SolarClockUnavailable,
    Error(ReplyError),
}
impl datom_codec::Datomic for Reply {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Accepted" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Accepted)
            }
            "Theme" => std::result::Result::Ok(Self::Theme(datom_codec::Carrying::body(v)?)),
            "Warmth" => std::result::Result::Ok(Self::Warmth(datom_codec::Carrying::body(v)?)),
            "Brightness" => std::result::Result::Ok(Self::Brightness(datom_codec::Carrying::body(v)?)),
            "State" => std::result::Result::Ok(Self::State(datom_codec::Carrying::body(v)?)),
            "SolarClock" => std::result::Result::Ok(Self::SolarClock(datom_codec::Carrying::body(v)?)),
            "SolarClockUnavailable" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::SolarClockUnavailable)
            }
            "Error" => std::result::Result::Ok(Self::Error(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for Reply {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Accepted => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Accepted").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Theme(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Theme").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Warmth(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Warmth").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Brightness(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Brightness").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::State(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("State").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::SolarClock(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("SolarClock").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::SolarClockUnavailable => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(
                        protos::Word::try_from("SolarClockUnavailable").expect("static variant"),
                    )
                    .expect("stable variant"),
                ),
                Self::Error(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Error").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyTheme(pub ThemeMode);
impl datom_codec::Datomic for ReplyTheme {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: ThemeMode = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ReplyTheme {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyWarmth(pub protos::Integer);
impl datom_codec::Datomic for ReplyWarmth {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ReplyWarmth {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyBrightness(pub protos::Integer);
impl datom_codec::Datomic for ReplyBrightness {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ReplyBrightness {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyState(pub ThemeMode, pub protos::Integer, pub protos::Integer);
impl datom_codec::Datomic for ReplyState {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 3)?;
        let p0: ThemeMode = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Integer = datom_codec::Positional::position(&mut p)?;
        let p2: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ReplyState {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplySolarClock(pub protos::Integer, pub protos::Integer);
impl datom_codec::Datomic for ReplySolarClock {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ReplySolarClock {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyError(pub protos::Text);
impl datom_codec::Datomic for ReplyError {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: protos::Text = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ReplyError {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}
impl datom_codec::Datomic for ThemeMode {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Dark" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Dark)
            }
            "Light" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Light)
            }
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemeMode {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Dark => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Dark").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Light => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Light").expect("static variant"))
                        .expect("stable variant"),
                ),
            },
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarmthLevel {
    Cold,
    Cool,
    Neutral,
    Warm,
    Warmer,
    Warmest,
}
impl datom_codec::Datomic for WarmthLevel {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Cold" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Cold)
            }
            "Cool" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Cool)
            }
            "Neutral" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Neutral)
            }
            "Warm" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Warm)
            }
            "Warmer" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Warmer)
            }
            "Warmest" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Warmest)
            }
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for WarmthLevel {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Cold => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Cold").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Cool => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Cool").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Neutral => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Neutral").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Warm => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Warm").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Warmer => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Warmer").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Warmest => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Warmest").expect("static variant"))
                        .expect("stable variant"),
                ),
            },
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrightnessLevel {
    Dim,
    Dimmer,
    Mid,
    Bright,
    Brighter,
    Brightest,
}
impl datom_codec::Datomic for BrightnessLevel {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Dim" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Dim)
            }
            "Dimmer" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Dimmer)
            }
            "Mid" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Mid)
            }
            "Bright" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Bright)
            }
            "Brighter" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Brighter)
            }
            "Brightest" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Brightest)
            }
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for BrightnessLevel {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Dim => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Dim").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Dimmer => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Dimmer").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Mid => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Mid").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Bright => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Bright").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Brighter => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Brighter").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Brightest => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Brightest").expect("static variant"))
                        .expect("stable variant"),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RampDuration {
    Minutes(protos::Integer),
    Seconds(protos::Integer),
}
impl datom_codec::Datomic for RampDuration {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Minutes" => std::result::Result::Ok(Self::Minutes(datom_codec::Carrying::body(v)?)),
            "Seconds" => std::result::Result::Ok(Self::Seconds(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for RampDuration {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Minutes(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Minutes").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Seconds(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Seconds").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config(pub ThemeAxis, pub WarmthAxis, pub BrightnessAxis);
impl datom_codec::Datomic for Config {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 3)?;
        let p0: ThemeAxis = datom_codec::Positional::position(&mut p)?;
        let p1: WarmthAxis = datom_codec::Positional::position(&mut p)?;
        let p2: BrightnessAxis = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2))
    }
}
impl protos::Conceivable<datom_codec::Datom> for Config {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeAxis(
    pub std::vec::Vec<ThemeConcern>,
    pub ThemePalettes,
    pub std::option::Option<protos::Text>,
    pub std::option::Option<protos::Integer>,
    pub std::option::Option<GhosttyConfigTemplates>,
    pub std::option::Option<PiThemeControl>,
    pub ThemeSchedule,
);
impl datom_codec::Datomic for ThemeAxis {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 7)?;
        let p0: std::vec::Vec<ThemeConcern> = datom_codec::Positional::position(&mut p)?;
        let p1: ThemePalettes = datom_codec::Positional::position(&mut p)?;
        let p2: std::option::Option<protos::Text> = datom_codec::Positional::position(&mut p)?;
        let p3: std::option::Option<protos::Integer> = datom_codec::Positional::position(&mut p)?;
        let p4: std::option::Option<GhosttyConfigTemplates> = datom_codec::Positional::position(&mut p)?;
        let p5: std::option::Option<PiThemeControl> = datom_codec::Positional::position(&mut p)?;
        let p6: ThemeSchedule = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2, p3, p4, p5, p6))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemeAxis {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.3).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.4).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.5).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.6).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeConcern {
    Terminal,
    Desktop,
    Ghostty,
    Pi,
}
impl datom_codec::Datomic for ThemeConcern {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Terminal" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Terminal)
            }
            "Desktop" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Desktop)
            }
            "Ghostty" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Ghostty)
            }
            "Pi" => {
                datom_codec::Headed::nothing(v)?;
                std::result::Result::Ok(Self::Pi)
            }
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemeConcern {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Terminal => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Terminal").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Desktop => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Desktop").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Ghostty => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Ghostty").expect("static variant"))
                        .expect("stable variant"),
                ),
                Self::Pi => datom_codec::Datom::Word(
                    datom_codec::DatomWord::try_from(protos::Word::try_from("Pi").expect("static variant"))
                        .expect("stable variant"),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemePalettes(pub ThemePalette, pub ThemePalette);
impl datom_codec::Datomic for ThemePalettes {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: ThemePalette = datom_codec::Positional::position(&mut p)?;
        let p1: ThemePalette = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemePalettes {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemePalette(
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
    pub protos::Text,
);
impl datom_codec::Datomic for ThemePalette {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 16)?;
        let p0: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p2: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p3: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p4: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p5: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p6: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p7: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p8: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p9: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p10: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p11: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p12: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p13: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p14: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p15: protos::Text = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2, p3, p4, p5, p6, p7, p8, p9, p10, p11, p12, p13, p14, p15))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemePalette {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.3).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.4).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.5).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.6).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.7).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.8).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.9).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.10).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.11).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.12).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.13).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.14).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.15).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhosttyConfigTemplates(pub protos::Text, pub protos::Text);
impl datom_codec::Datomic for GhosttyConfigTemplates {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: protos::Text = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Text = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for GhosttyConfigTemplates {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiThemeControl(
    pub PiThemeControlRegistryDirectory,
    pub std::option::Option<protos::Integer>,
    pub std::option::Option<protos::Integer>,
);
impl datom_codec::Datomic for PiThemeControl {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 3)?;
        let p0: PiThemeControlRegistryDirectory = datom_codec::Positional::position(&mut p)?;
        let p1: std::option::Option<protos::Integer> = datom_codec::Positional::position(&mut p)?;
        let p2: std::option::Option<protos::Integer> = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2))
    }
}
impl protos::Conceivable<datom_codec::Datom> for PiThemeControl {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PiThemeControlRegistryDirectory {
    RuntimeRelative(protos::Text),
    Absolute(protos::Text),
}
impl datom_codec::Datomic for PiThemeControlRegistryDirectory {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "RuntimeRelative" => std::result::Result::Ok(Self::RuntimeRelative(datom_codec::Carrying::body(v)?)),
            "Absolute" => std::result::Result::Ok(Self::Absolute(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for PiThemeControlRegistryDirectory {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::RuntimeRelative(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("RuntimeRelative").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Absolute(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Absolute").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeSchedule {
    Manual(ThemeMode),
    Scheduled(ThemeScheduleScheduled),
}
impl datom_codec::Datomic for ThemeSchedule {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Manual" => std::result::Result::Ok(Self::Manual(datom_codec::Carrying::body(v)?)),
            "Scheduled" => std::result::Result::Ok(Self::Scheduled(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemeSchedule {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Manual(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Manual").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Scheduled(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Scheduled").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeScheduleScheduled(pub std::vec::Vec<ThemeWaypoint>, pub ThemeMode);
impl datom_codec::Datomic for ThemeScheduleScheduled {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: std::vec::Vec<ThemeWaypoint> = datom_codec::Positional::position(&mut p)?;
        let p1: ThemeMode = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemeScheduleScheduled {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeWaypoint(pub RampTrigger, pub ThemeMode);
impl datom_codec::Datomic for ThemeWaypoint {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: RampTrigger = datom_codec::Positional::position(&mut p)?;
        let p1: ThemeMode = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for ThemeWaypoint {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarmthAxis(pub WarmthSchedule);
impl datom_codec::Datomic for WarmthAxis {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: WarmthSchedule = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for WarmthAxis {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WarmthSchedule {
    Manual(WarmthLevel),
    Scheduled(WarmthScheduleScheduled),
}
impl datom_codec::Datomic for WarmthSchedule {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Manual" => std::result::Result::Ok(Self::Manual(datom_codec::Carrying::body(v)?)),
            "Scheduled" => std::result::Result::Ok(Self::Scheduled(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for WarmthSchedule {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Manual(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Manual").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Scheduled(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Scheduled").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarmthScheduleScheduled(pub std::vec::Vec<WarmthWaypoint>, pub WarmthLevel);
impl datom_codec::Datomic for WarmthScheduleScheduled {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: std::vec::Vec<WarmthWaypoint> = datom_codec::Positional::position(&mut p)?;
        let p1: WarmthLevel = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for WarmthScheduleScheduled {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarmthWaypoint(pub RampTrigger, pub WarmthLevel, pub RampDuration);
impl datom_codec::Datomic for WarmthWaypoint {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 3)?;
        let p0: RampTrigger = datom_codec::Positional::position(&mut p)?;
        let p1: WarmthLevel = datom_codec::Positional::position(&mut p)?;
        let p2: RampDuration = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2))
    }
}
impl protos::Conceivable<datom_codec::Datom> for WarmthWaypoint {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrightnessAxis(pub BrightnessSchedule);
impl datom_codec::Datomic for BrightnessAxis {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: BrightnessSchedule = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0))
    }
}
impl protos::Conceivable<datom_codec::Datom> for BrightnessAxis {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrightnessSchedule {
    Manual(BrightnessLevel),
    Scheduled(BrightnessScheduleScheduled),
}
impl datom_codec::Datomic for BrightnessSchedule {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Manual" => std::result::Result::Ok(Self::Manual(datom_codec::Carrying::body(v)?)),
            "Scheduled" => std::result::Result::Ok(Self::Scheduled(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for BrightnessSchedule {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Manual(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Manual").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Scheduled(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Scheduled").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrightnessScheduleScheduled(pub std::vec::Vec<BrightnessWaypoint>, pub BrightnessLevel);
impl datom_codec::Datomic for BrightnessScheduleScheduled {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: std::vec::Vec<BrightnessWaypoint> = datom_codec::Positional::position(&mut p)?;
        let p1: BrightnessLevel = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for BrightnessScheduleScheduled {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrightnessWaypoint(pub RampTrigger, pub BrightnessLevel, pub RampDuration);
impl datom_codec::Datomic for BrightnessWaypoint {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 3)?;
        let p0: RampTrigger = datom_codec::Positional::position(&mut p)?;
        let p1: BrightnessLevel = datom_codec::Positional::position(&mut p)?;
        let p2: RampDuration = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1, p2))
    }
}
impl protos::Conceivable<datom_codec::Datom> for BrightnessWaypoint {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.2).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RampTrigger {
    Sunrise(protos::Integer),
    Sunset(protos::Integer),
    CivilDawn(protos::Integer),
    CivilDusk(protos::Integer),
    TimeOfDay(RampTriggerTimeOfDay),
}
impl datom_codec::Datomic for RampTrigger {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Sunrise" => std::result::Result::Ok(Self::Sunrise(datom_codec::Carrying::body(v)?)),
            "Sunset" => std::result::Result::Ok(Self::Sunset(datom_codec::Carrying::body(v)?)),
            "CivilDawn" => std::result::Result::Ok(Self::CivilDawn(datom_codec::Carrying::body(v)?)),
            "CivilDusk" => std::result::Result::Ok(Self::CivilDusk(datom_codec::Carrying::body(v)?)),
            "TimeOfDay" => std::result::Result::Ok(Self::TimeOfDay(datom_codec::Carrying::body(v)?)),
            _ => std::result::Result::Err(datom_codec::Headed::reject(
                &v,
                datom_codec::Problem::UnknownVariant(protos::Word::try_from(v.name).expect("variant name")),
            )),
        }
    }
}
impl protos::Conceivable<datom_codec::Datom> for RampTrigger {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            match self {
                Self::Sunrise(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Sunrise").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::Sunset(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("Sunset").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::CivilDawn(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("CivilDawn").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::CivilDusk(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("CivilDusk").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
                Self::TimeOfDay(p0) => datom_codec::Datom::Variant(
                    protos::Symbol::try_from("TimeOfDay").expect("static variant"),
                    std::boxed::Box::new(protos::Conceivable::conceive(p0).expect("infallible datom ascent").1),
                ),
            },
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RampTriggerTimeOfDay(pub protos::Integer, pub protos::Integer);
impl datom_codec::Datomic for RampTriggerTimeOfDay {
    fn incorporate(site: datom_codec::Site<'_>) -> std::result::Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Integer = datom_codec::Positional::position(&mut p)?;
        std::result::Result::Ok(Self(p0, p1))
    }
}
impl protos::Conceivable<datom_codec::Datom> for RampTriggerTimeOfDay {
    type Fault = std::convert::Infallible;
    fn conceive(&self) -> std::result::Result<protos::Situated<datom_codec::Datom>, Self::Fault> {
        std::result::Result::Ok(protos::Situated(
            protos::Situation { extent: protos::Extent(0, 0), children: vec![] },
            datom_codec::Datom::Struct(vec![
                protos::Conceivable::conceive(&self.0).expect("infallible datom ascent").1,
                protos::Conceivable::conceive(&self.1).expect("infallible datom ascent").1,
            ]),
        ))
    }
}
