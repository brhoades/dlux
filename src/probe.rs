use lib::{config, device::Displays, types::*};
use log::info;

pub async fn run() -> Result<()> {
    env_logger::init();
    let mut disps = Displays::new(vec![config::DeviceConfig::default()])?;

    for disp in disps.iter_mut() {
        let edid = disp.display_info();
        info!("device: {}", disp);
        info!("edid: {:?}", edid);
    }

    Ok(())
}
