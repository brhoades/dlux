use ddc::{Edid};

use crate::types::*;

#[derive(Debug, Default, Clone)]
pub struct DisplayInfo {
    manufacturer: String,
    model: String,
    serial: String,
}

impl DisplayInfo {
    pub(crate) fn new<F: std::error::Error, T: Edid<EdidError = F>>(d: &mut T) -> Result<DisplayInfo> {
        let mut edid = vec![0; 128];
        match d.read_edid(0, &mut edid).map_err(|e| format_err!("error reading device EDID: {}", e)) {
            Err(e) => Err(format_err!("error reading device EDID: {}", e)),
            Ok(128) => Ok(()),
            Ok(size) => Err(format_err!("read insufficient data from device EDID: got {} bytes, wanted {}", size, edid.capacity())),
        }?;

        let mut info = DisplayInfo::default();
        let descrs = vec![&edid[54..72], &edid[72..90], &edid[90..108], &edid[108..126]];

        for descr in descrs {
            match read_descriptor(descr)? {
                DispDescr::Serial(srl) => info.serial = srl,
                DispDescr::Model(model) => info.model = model,
                _ => (),
            }
        }

        Ok(info)
    }
}


enum DispDescr {
    Serial(String),
    Model(String),
    Other,
}

fn read_descriptor(descr: &[u8]) -> Result<DispDescr> {
    if descr.len() != 18 {
        return Err(format_err!("invalid descriptor length: expected 18, got {}", descr.len()));
    }


    // https://en.wikipedia.org/wiki/Extended_Display_Identification_Data#Display_Descriptors
    Ok(match &descr[3] {
        0xff => DispDescr::Serial(std::str::from_utf8(&descr[5..18])?.trim().to_owned()),
        0xfc => DispDescr::Model(std::str::from_utf8(&descr[5..18])?.trim().to_owned()),
        _ => DispDescr::Other,
    })
}
