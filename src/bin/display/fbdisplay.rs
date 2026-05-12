use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use ratatui::style::Stylize;
use ratatui::widgets::{Block, Paragraph};

pub fn fbdisplay() -> std::io::Result<()> {
    let mut fb = OpenOptions::new().write(true).open("/dev/fb0")?;

    let mut file = File::open("logo.bmp")?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let data_offset = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]) as usize;

    let width = u32::from_le_bytes([buf[18], buf[19], buf[20], buf[21]]) as usize;

    let height = u32::from_le_bytes([buf[22], buf[23], buf[24], buf[25]]) as usize;

    let pixels = &buf[data_offset..];

    let fb_width = 240;
    let fb_height = 240;
    let stride = 480;

    let mut framebuffer = vec![0u8; fb_height * stride];

    let row_size = width * 2;

    for y in 0..fb_height {
        if y >= height {
            break;
        }

        for x in 0..fb_width {
            if x >= width {
                break;
            }

            let bmp_index = (height - 1 - y) * row_size + x * 2;

            let lo = pixels[bmp_index];
            let hi = pixels[bmp_index + 1];

            let fb_index = y * stride + x * 2;

            framebuffer[fb_index] = lo;
            framebuffer[fb_index + 1] = hi;
        }
    }

    fb.seek(SeekFrom::Start(0))?;
    fb.write_all(&framebuffer)?;

    ratatui::run(|terminal| {
        terminal.draw(|frame| {
            let block = Block::bordered().title("Welcome");
            let greeting = Paragraph::new("Hello, Ratatui! 🐭")
                .centered()
                .yellow()
                .block(block);
            frame.render_widget(greeting, frame.area());
        })?;
        std::thread::sleep(std::time::Duration::from_secs(5));
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    Ok(())
}
