use display_interface_spi::SPIInterfaceNoCS; // NEW IMPORT
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_9X15_BOLD},
    prelude::*,
    text::Text,
};
use linux_embedded_hal::{
    Delay, Pin, Spidev,
    spidev::{SpiModeFlags, SpidevOptions},
    sysfs_gpio::Direction,
};
use st7789::{Orientation, ST7789};
use tinybmp::Bmp;

pub fn display() {
    println!("Initializing SPI3-M1...");

    // 1. Open and Configure the SPI Pipe
    let mut spi = Spidev::open("/dev/spidev3.0").expect("Failed to open SPI device");
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(40_000_000)
        .mode(SpiModeFlags::SPI_MODE_3)
        .build();
    spi.configure(&options).expect("Failed to configure SPI");

    // 2. Configure DC and RST Pins
    println!("Exporting GPIO 113 (DC) and 106 (RST)...");

    let dc = Pin::new(113);
    if !dc.is_exported() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // This will now crash and tell us WHY it failed, instead of hanging forever
        dc.export().expect("FATAL: Failed to export DC pin (113)");
        // Give the Linux file system 100ms to physically create the system files
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    dc.set_direction(Direction::Out)
        .expect("Failed to set DC direction");

    let rst = Pin::new(106);
    if !rst.is_exported() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // This will now crash and tell us WHY it failed, instead of hanging forever
        rst.export().expect("FATAL: Failed to export RST pin (106)");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    rst.set_direction(Direction::Out)
        .expect("Failed to set RST direction");

    // 3. Initialize the Display Interface and Driver
    println!("Initializing ST7789 Driver...");
    let mut delay = Delay {};

    // THE FIX: Wrap the SPI pipe and DC pin into a unified Display Interface
    let di = SPIInterfaceNoCS::new(spi, dc);

    // Pass the interface, reset pin (as an Option), backlight (None), and screen size
    let mut display = ST7789::new(di, rst, 240, 240);

    display
        .init(&mut delay)
        .expect("Failed to initialize display");
    display
        .set_orientation(Orientation::Portrait)
        .expect("Failed to set orientation");

    // 4. Draw the Image!
    println!("Drawing image...");
    display.clear(Rgb565::BLACK).unwrap();

    // 1. Load the raw bytes of the file directly into memory
    let bmp_data = include_bytes!("logo.bmp");

    // 2. Parse the bytes into an RGB565 Image format
    let bmp = Bmp::<Rgb565>::from_slice(bmp_data).expect("Failed to parse BMP data");

    // 3. Wrap it in a drawable Image object and set its starting coordinate (X: 0, Y: 0)
    let image = Image::new(&bmp, Point::zero());

    // 4. Push the pixels to the screen via the SPI pipe
    image.draw(&mut display).unwrap();

    println!("Done!");
}
