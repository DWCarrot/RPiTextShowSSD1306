use ssd1306::displaysize::DisplaySize;
use ssd1306::prelude::WriteOnlyDataCommand;
use ssd1306::mode::GraphicsMode;
use embedded_graphics::prelude::*;
use embedded_graphics::drawable::Drawable;
use embedded_graphics::style::TextStyle;
use embedded_graphics::fonts::Text;
use embedded_graphics::fonts::Font;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::geometry::Point;
use async_std::io as aio;
use crate::manager::Canvas;
use crate::contents::Page;

pub struct SSD1306Display<DI: WriteOnlyDataCommand, DSIZE: DisplaySize, F: Font> {
    display: GraphicsMode<DI, DSIZE>,
    style: TextStyle<BinaryColor, F>,
    line_interval: i32,
}

impl<DI: WriteOnlyDataCommand, DSIZE: DisplaySize, F: Font + Copy> Canvas for SSD1306Display<DI, DSIZE, F> {
    
    fn draw(&mut self, page: &Page) -> aio::Result<()> {
        match page {
            Page::Empty => {},
            Page::Text{ lines } => {
                self.display.clear();
                let mut point = Point::zero();
                for line in lines {
                    Text::new(line.as_str(), point)
                        .into_styled(self.style)
                        .draw(&mut self.display)
                        .map_err(|e| aio::Error::new(aio::ErrorKind::Other, format!("{:?}", e)))?;
                    point.y += self.line_interval;
                }
            },
            Page::BImage{ data, w, h } => {

            }
        }
        Ok(())
    }

    fn init(&mut self) -> aio::Result<()> {
        self.display.init().map_err(|e| aio::Error::new(aio::ErrorKind::Other, format!("{:?}", e)))
    }

    fn flush(&mut self) -> aio::Result<()> {
        self.display.flush().map_err(|e| aio::Error::new(aio::ErrorKind::Other, format!("{:?}", e)))
    }

    fn clear(&mut self) -> aio::Result<()> {
        self.display.clear();
        Ok(())
    }
}

impl<DI: WriteOnlyDataCommand, DSIZE: DisplaySize, F: Font> SSD1306Display<DI, DSIZE, F> {
    
    pub fn new(display: GraphicsMode<DI, DSIZE>, style: TextStyle<BinaryColor, F>, line_interval: u32) -> Self {
        SSD1306Display {
            display,
            style,
            line_interval: line_interval as i32,
        }
    }
}