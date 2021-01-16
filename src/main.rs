mod contents;
mod operation;
mod server;
mod manager;
mod display;

use std::time::Duration;
use async_std::task;
use async_std::io as aio;
use linux_embedded_hal::I2cdev;
use ssd1306::builder::I2CDIBuilder;
use ssd1306::displaysize;
use ssd1306::Builder;
use embedded_graphics::fonts;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::style::TextStyleBuilder;
use server::Server;
use manager::Canvas;
use manager::Manager;
use contents::Page;
use contents::Content;
use clap::App;
use clap::Arg;


macro_rules! build0 {
    ($config:ident, $interface:ident, $size:expr, [$($font:expr=>$fontN:expr),+]) => {
        {
            let display = Builder::new().size($size).connect($interface).into();
            match $config.font {
            $(
                $font => {
                    let text_style = TextStyleBuilder::new($fontN)
                        .text_color(BinaryColor::On)
                        .background_color(BinaryColor::Off)
                        .build();
                    let canvas = display::SSD1306Display::new(display, text_style, $config.line_interval);
                    Box::new(canvas)
                }
            )+
                _ => {
                    return Err(aio::Error::new(aio::ErrorKind::Other, format!("unsupport font:{}", $config.font)))
                }
            }
        }
    };
    ($config:ident, $interface:ident, ($($size:expr=>$sizeN:expr),+), [$($font:expr=>$fontN:expr),+]) => {
        {
            match $config.display_size {
            $(
                $size => {
                    build0!($config, $interface, $sizeN, [$($font=>$fontN),+])
                }
            )+
                _ => {
                    return Err(aio::Error::new(aio::ErrorKind::Other, format!("unsupport display-size:{}", $config.display_size)))
                }
            }
        }
    }
}

const NAME: &'static str = env!("CARGO_PKG_NAME");
const DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");

fn main() {

    let matches = App::new(NAME)
        .about(DESCRIPTION)
        .version(VERSION)
        .author(AUTHORS)
        .arg(
            Arg::with_name("size")
                .short("s")
                .long("size")
                .help("display-size, can be `128x32`, `128x64`, `96x16`")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("font")
                .short("f")
                .long("font")
                .help("font, can be `6x8`, `6x12`, `8x12`, `8x16`")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("line_interval")
                .short("p")
                .long("line_interval")
                .help("intervals between lines")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("page_roll_interval")
                .short("r")
                .long("page_roll_interval")
                .help("time for each page to stay, in millisecond")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("bind")
                .required(true)
                .index(1)
        )
        .get_matches();

    let config = Config {
        bind: matches.value_of("bind").expect("bind"),
        page_roll_interval: matches.value_of("page_roll_interval").expect("page_roll_interval").parse().unwrap(),
        display_size: matches.value_of("size").expect("size"),
        font: matches.value_of("font").expect("font"),
        line_interval: matches.value_of("line_interval").expect("line_interval").parse().unwrap(),
    };
    task::block_on(server(config)).unwrap();
}

async fn server<'a>(config: Config<'a>) -> aio::Result<()> {
    
    let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    let interface = I2CDIBuilder::new().init(i2c);

    let canvas: Box<dyn Canvas> = match config.display_size {
        "128x32" => {
            build0!(
                config, 
                interface, 
                displaysize::DisplaySize128x32,
                ["6x8"=>fonts::Font6x8, "6x12"=>fonts::Font6x12, "8x16"=>fonts::Font8x16, "12x16"=>fonts::Font12x16]
            )
        }
        "128x64" => {
            build0!(
                config,
                interface, 
                displaysize::DisplaySize128x64,
                ["6x8"=>fonts::Font6x8, "6x12"=>fonts::Font6x12, "8x16"=>fonts::Font8x16, "12x16"=>fonts::Font12x16]
            )
        }
        "96x16" => {
            build0!(
                config, 
                interface, 
                displaysize::DisplaySize96x16,
                ["6x8"=>fonts::Font6x8, "6x12"=>fonts::Font6x12, "8x16"=>fonts::Font8x16, "12x16"=>fonts::Font12x16]
            )
        }
        _ => {
            return Err(aio::Error::new(aio::ErrorKind::Other, format!("unsupport display-size:{}", config.display_size)))
        }
    };


    let mgr = Manager::new(Content::new(4), canvas)?;
    let server = Server::new(mgr, Duration::from_millis(config.page_roll_interval as u64), 1024, 4);
    server.start_server(config.bind).await?;
    Ok(())
}

pub struct Print;

impl Canvas for Print {

    fn draw(&mut self, page: &Page) -> aio::Result<()> {
        println!("{:?}", page);
        Ok(())
    }

    fn init(&mut self) -> aio::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> aio::Result<()> {
        Ok(())
    }

    fn clear(&mut self) -> aio::Result<()>  {
        Ok(())
    }
}

pub struct Config<'a> {
    bind: &'a str,
    page_roll_interval: u32,
    display_size: &'a str,
    font: &'a str,
    line_interval: u32
}

impl<'a> Default for Config<'a> {

    fn default() -> Self {
        Config {
            bind: "127.0.0.1:17900",
            page_roll_interval: 5000,
            display_size: "128x32",
            font: "6x8",
            line_interval: 10
        }
    }
}