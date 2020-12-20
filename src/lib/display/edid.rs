use ddc::Edid;

use crate::types::*;

#[derive(Debug, Default, Clone)]
pub struct DeviceInfo {
    pub(crate) manufacturer: String,
    pub(crate) model: String,
    pub(crate) serial: String,
}

impl DeviceInfo {
    pub fn new<F: std::error::Error, T: Edid<EdidError = F>>(d: &mut T) -> Result<DeviceInfo> {
        let mut edid = vec![0; 128];
        match d
            .read_edid(0, &mut edid)
            .map_err(|e| format_err!("error reading device EDID: {}", e))
        {
            Err(e) => Err(format_err!("error reading device EDID: {}", e)),
            Ok(128) => Ok(()),
            Ok(size) => Err(format_err!(
                "read insufficient data from device EDID: got {} bytes, wanted {}",
                size,
                edid.capacity()
            )),
        }?;

        let mut info = DeviceInfo::default();
        let descrs = vec![
            &edid[54..72],
            &edid[72..90],
            &edid[90..108],
            &edid[108..126],
        ];

        for descr in descrs {
            match read_descriptor(descr)? {
                DispDescr::Serial(srl) => info.serial = srl,
                DispDescr::Model(model) => info.model = model,
                _ => (),
            }
        }

        info.manufacturer = read_mfg_id(&edid[8..=9])?;
        Ok(info)
    }
}

/// read_mfg_id expects edid bytes 8 & 9 and returns the alphabetical manufacturer.
///
/// bitfield: 0011 0111 0100 1001
/// encoded:  0111 1122 2223 3333
/// gives:    0001 1111 0002 2222 0003 3333
///
/// bit 15 is zero
/// byte 1, bits 2-6 on the are the first letter.
/// byte 1 bit 1, byte 2 bit 5-9 are the second letter.
/// byte 2, bit 0-4 are the third letter.
fn read_mfg_id(edid: &[u8]) -> Result<String> {
    if edid.len() != 2 {
        return Err(format_err!(
            "expected two bytes to read the manufacturer id, got {}",
            edid.len()
        ));
    }

    let res = &[
        (edid[0] & 0x7D) >> 2,
        ((edid[0] & 0x3) << 3) | ((edid[1] & 0xE0) >> 5),
        edid[1] & 0x1F,
    ];

    // 0x0 is A, 0x1 is B, etc. Add 65 to map to ascii.
    Ok(std::str::from_utf8(&res.iter().map(|c| c + 65 - 1).collect::<Vec<_>>())?.to_owned())
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} (SN: {})",
            self.manufacturer, self.model, self.serial
        )
    }
}

enum DispDescr {
    Serial(String),
    Model(String),
    Other,
}

fn read_descriptor(descr: &[u8]) -> Result<DispDescr> {
    if descr.len() != 18 {
        return Err(format_err!(
            "invalid descriptor length: expected 18, got {}",
            descr.len()
        ));
    }

    // https://en.wikipedia.org/wiki/Extended_Display_Identification_Data#Display_Descriptors
    Ok(match &descr[3] {
        0xff => DispDescr::Serial(std::str::from_utf8(&descr[5..18])?.trim().to_owned()),
        0xfc => DispDescr::Model(std::str::from_utf8(&descr[5..18])?.trim().to_owned()),
        _ => DispDescr::Other,
    })
}

#[test]
fn test_parse_mfg_example() {
    env_logger::init();
    let res = read_mfg_id(&[0x24, 0x4D]);

    assert_eq!("IBM", res.unwrap());
}
