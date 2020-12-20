use regex::Regex;

use crate::{logging::*, prelude::*};

#[derive(Default, Debug, Clone)]
pub struct DeviceMatcher {
    pub(crate) model: Option<Regex>,
    pub(crate) mfg: Option<Regex>,
    pub(crate) serial: Option<String>,
}

impl DeviceMatcher {
    /// Compares the current device matcher to the provided manufacturer, model,
    /// and serial. Returns if there is a match.
    pub fn matches(&self, info: &DeviceInfo) -> bool {
        let mtches = match &self {
            Self {
                model: None,
                mfg: None,
                serial: None,
            } => return true,
            Self {
                model: _,
                mfg: _,
                serial: Some(exact_serial),
            } => return *exact_serial == info.serial,
            Self {
                model: Some(re_model),
                mfg: None,
                serial: None,
            } => vec![re_model.find(&info.model)],
            Self {
                model: None,
                mfg: Some(re_mfg),
                serial: None,
            } => vec![re_mfg.find(&info.manufacturer)],
            Self {
                model: Some(re_model),
                mfg: Some(re_mfg),
                serial: None,
            } => vec![re_mfg.find(&info.manufacturer), re_model.find(&info.model)],
        };

        if mtches.len() == 0 || mtches.iter().all(Option::is_none) {
            debug!("{} does not match {}", info, self.internal_fmt());
            trace!("matchers output: {:#?}", info);
            return false;
        }

        debug!("{} {}", info, self);
        true
    }

    fn internal_fmt(&self) -> String {
        match &self {
            Self {
                model: None,
                mfg: None,
                serial: None,
            } => "any device".to_owned(),
            Self {
                model: _,
                mfg: _,
                serial: Some(serial),
            } => format!("serial {}", serial),
            Self {
                model: Some(model),
                mfg: None,
                serial: None,
            } => format!("model {}", model),
            Self {
                model: None,
                mfg: Some(mfg),
                serial: None,
            } => format!("manufacturer {}", mfg),
            Self {
                model: Some(model),
                mfg: Some(mfg),
                serial: None,
            } => format!("model {} and manufacturer {}", model, mfg),
        }
    }
}

impl std::fmt::Display for DeviceMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "matches {}", self.internal_fmt())
    }
}
