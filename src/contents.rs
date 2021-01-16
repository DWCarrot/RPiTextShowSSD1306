#[derive(Debug, Clone)]
pub enum Page {
    Empty,
    Text { lines: Vec<String> },
    BImage { data: Box<[u8]>, w: u32, h: u32 },
}

impl Page {

    pub fn new_text(line_num: usize) -> Self {
        Self::Text {
            lines: (0 .. line_num).map(|_i| String::default()).collect()
        }
    }

    pub fn new_image(w: u32, h: u32) -> Self {
        Self::BImage {
            data: (0 .. ((w + 7) / 8) * h).map(|i| 0u8).collect(),
            w,
            h
        }
    }

    pub fn set_text(&mut self, i: usize, text: String) -> bool {
        if let Self::Text{ lines } = self {
            if let Some(line) = lines.get_mut(i) {
                *line = text;
                return true;
            }
        }
        false
    }

    pub fn get_text(&self, i: usize) -> Option<&str> {
        if let Self::Text{ lines } = self {
            return lines.get(i).map(String::as_str);
        }
        None
    }

    pub fn remove_text(&mut self, i: usize) -> bool {
        if let Self::Text{ lines } = self {
            if i < lines.len() {
                lines.remove(i);
                return true;
            }
        }
        false
    }

    pub fn line_num(&self) -> usize {
        if let Self::Text{ lines } = self {
            lines.len()
        } else {
            0
        }
    }
}


pub struct Content {
    pages: Vec<Page>,
    line_limit: usize
}

impl Content {

    pub fn new(line_limit: usize) -> Self {
        Content {
            pages: Vec::new(),
            line_limit,
        }
    }

    pub fn new_with_capacity(line_limit: usize, capacity: usize) -> Self {
        Content {
            pages: Vec::with_capacity(capacity),
            line_limit
        }
    }

    pub fn line_limit(&self) -> usize {
        self.line_limit
    }

    pub fn get(&self, i: usize) -> Option<&Page> {
        self.pages.get(i)
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut Page> {
        self.pages.get_mut(i)
    }

    pub fn get_mut_or_add<G: FnOnce()->Page>(&mut self, i: usize, generate: G) -> Option<&mut Page> {
        let len = self.pages.len();
        if i > len {
            None
        } else {
            if i == len {
                let page = generate();
                self.pages.push(page);
            } 
            Some(unsafe{ self.pages.get_unchecked_mut(i) })
        }
    }

    pub fn set(&mut self, i: usize, page: Page) -> bool {
        let len = self.pages.len();
        if i > len {
            false
        } else {
            if i == len {
                self.pages.push(page);
            } else {
                unsafe {
                    *(self.pages.get_unchecked_mut(i)) = page;
                }
            }
            true
        }
    }

    pub fn remove(&mut self, i: usize) -> Option<Page> {
        if i < self.pages.len() {
            Some(self.pages.remove(i))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }
}
