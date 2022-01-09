use mobile_device as md;

fn main() {
    env_logger::init();

    let devices = md::get_device_list();
    println!("Connected devices: {}", devices.len());
    for device in devices {
        println!("{:?}", device);
    }
}
