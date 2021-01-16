use std::error;
use std::fmt;
use base64::DecodeError;
use crate::contents::Page;
use crate::contents::Content;

/**
 * `hello world` set text "hello world"
 * `@1:2+hello world` set page 1 line 2 = "hello world"
 * `@1:2~`  delete page 1 line 2
 * `@1:2?` query page 1 line 2
 * `@1+128,32:<base64>` set page 1 with width=128,height=32, decode data from base64
 * `@1~` delete page 1
 */

const SP_PAGE: u8 = '@' as u8;
const SP_LINE: u8 = ':' as u8;
const OP_DATA: u8 = '+' as u8;
const SP_SIZE: u8 = ',' as u8;
const OP_STRING: u8 = '+' as u8;
const OP_BASE64: u8 = ':' as u8;
const OP_DELETE: u8 = '~' as u8;
const OP_QUERY: u8 = '?' as u8;
const CRLF: (u8, u8) = ('\r' as u8, '\n' as u8);

#[derive(Debug)]
pub enum Operation {
    Pass,
    RSetText(String),
    SetText {
        page: usize,
        line: usize,
        text: String,
    },
    DeleteText {
        page: usize,
        line: usize,
    },
    QueryText {
        page: usize,
        line: usize,
    },
    SetPage {
        page: usize,
        data: Vec<u8>,
        width: u32,
        height: u32,
    },
    DeletePage {
        page: usize,
    },
    QueryPage {
        page: usize,
    },
}

impl Operation {

    pub fn new(buf: impl AsRef<[u8]>) -> Result<Operation, ParseError> {

        let data = buf.as_ref();
        let n = data.len();
        if n < 2 {
            return Err(ParseError::InvalidLine);
        }

        let get_c = |i: usize| -> u8 {
            unsafe { *data.get_unchecked(i) }
        };

        if get_c(n-2) != CRLF.0 || get_c(n-1) != CRLF.1 {
            return Err(ParseError::InvalidLine);
        }

        let parse_uint = |i: usize| -> (usize, usize) {
            const RANGE: (u8, u8) = ('0' as u8, '9' as u8);
            let mut r = 0;
            let mut count = 0;
            for c in &data[i..] {
                let c = *c;
                if c < RANGE.0 || c > RANGE.1 {
                    return (r, count);
                }
                r *= 10;
                r += (c - RANGE.0) as usize;
                count += 1;
            }
            unreachable!();
        };

        let get_s = |i: usize| -> &[u8] {
            let j = data.len() - 2;
            &data[i..j]
        };

        if n == 2 {
            return Ok(Operation::Pass);
        }

        let mut i = 0;
        match get_c(i) {

            SP_PAGE => {
                i += 1;
                let (page, count) = parse_uint(i);
                if count == 0 {
                    return Err(ParseError::InvalidData(i));
                }
                i += count;

                match get_c(i) {

                    SP_LINE => {
                        i += 1;
                        let (line, count) = parse_uint(i);
                        if count == 0 {
                            return Err(ParseError::InvalidData(i));
                        }
                        i += count;

                        match get_c(i) {
                            OP_STRING => {
                                i += 1;
                                let text = String::from_utf8_lossy(get_s(i)).into_owned();
                                return Ok(Operation::SetText{ page, line, text });
                            },
                            OP_DELETE => {
                                i += 1;
                                return Ok(Operation::DeleteText{ page, line });
                            },
                            OP_QUERY => {
                                i += 1;
                                return Ok(Operation::QueryText{ page, line });
                            },
                            _ => {
                                return Err(ParseError::InvalidToken(i));
                            }
                        }
                    },

                    OP_DATA => {
                        i += 1;
                        
                        let (width, count) = parse_uint(i);
                        if count == 0 {
                            return Err(ParseError::InvalidData(i));
                        }
                        i += count;
                        
                        let c = get_c(i);
                        if c != SP_SIZE {
                            return Err(ParseError::InvalidToken(i));
                        }
                        i += 1;

                        let (height, count) = parse_uint(i);
                        if count == 0 {
                            return Err(ParseError::InvalidData(i));
                        }
                        i += count;

                        let c = get_c(i);
                        if c != OP_BASE64 {
                            return Err(ParseError::InvalidToken(i));
                        }
                        i += 1;

                        let data = base64::decode(get_s(i)).map_err(|e| ParseError::InvalidBase64(i, e))?;
                        return Ok(Operation::SetPage{ page, width: width as u32, height: height as u32, data});
                    },

                    OP_DELETE => {
                        i += 1;
                        return Ok(Operation::DeletePage{ page });
                    },

                    _ => {
                        i += 1;
                        return Err(ParseError::InvalidToken(i))
                    }
                }
            },

            _ => {
                let text = String::from_utf8_lossy(get_s(i)).into_owned();
                return Ok(Operation::RSetText(text));
            }
        }
    }

    pub fn modify(self, content: &mut Content) -> Result<QueryData, OperationError> {
        match self {
            Self::Pass => Ok(QueryData::None),
            Self::SetText{ page, line, text } => {
                let line_limit = content.line_limit();
                if line < line_limit {
                    if let Some(page) = content.get_mut_or_add(page, || { Page::new_text(line_limit) }) {
                        page.set_text(line, text);
                        Ok(QueryData::None)
                    } else {
                        Err(OperationError::PageOutOfBound(page, content.len()))
                    }
                } else {
                    Err(OperationError::LineOutOfPage(line, line_limit))
                }                
            },
            Self::DeleteText{ page, line } => {
                if let Some(page) = content.get_mut(page) {
                    if page.remove_text(line) {
                        Ok(QueryData::None)
                    } else {
                        Err(OperationError::LineOutOfPage(line, page.line_num()))
                    }
                } else {
                    Err(OperationError::PageOutOfBound(page, content.len()))
                }
            },
            Self::QueryText{ page, line } => {
                if let Some(page) = content.get(page) {
                    if let Some(s) = page.get_text(line) {
                        Ok(QueryData::Text(s))
                    } else {
                        Err(OperationError::LineOutOfPage(line, page.line_num()))
                    }
                } else {
                    Err(OperationError::PageOutOfBound(page, content.len()))
                }
            },
            Self::RSetText(text) => {
                let line_limit = content.line_limit();
                if 0 < line_limit {
                    if let Some(page) = content.get_mut_or_add(0, || { Page::new_text(line_limit) }) {
                        page.set_text(0, text);
                        Ok(QueryData::None)
                    } else {
                        Err(OperationError::PageOutOfBound(0, content.len()))
                    }
                } else {
                    Err(OperationError::LineOutOfPage(0, line_limit))
                }
            },
            Self::SetPage{ page, data, width, height } => {
                Err(OperationError::Invalid)
            },
            Self::DeletePage{ page } => {
                if let Some(page) = content.remove(page) {
                    Ok(QueryData::None)
                } else {
                    Err(OperationError::PageOutOfBound(0, content.len()))
                }
            },
            Self::QueryPage{ page } => {
                Err(OperationError::Invalid)
            }
        }
    }
}

pub struct CommandSplit<'a> {
    data: &'a [u8],
    index: &'a mut usize,
}

impl<'a> CommandSplit<'a> {

    pub fn new(data: &'a[u8], index: &'a mut usize) -> Self {
        CommandSplit {
            data,
            index,
        }
    }
}

impl<'a> Iterator for CommandSplit<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let mut i = *self.index;
        let data = self.data;
        while i < data.len() - 1 {
            let c = unsafe{ *data.get_unchecked(i) };
            if c == CRLF.0 {
                let c = unsafe{ *data.get_unchecked(i + 1) };
                if c == CRLF.1 {
                    let j = i + 2;
                    let i = *self.index;
                    *self.index = j;
                    return Some(&data[i..j]);
                }
            }
            i += 1;
        }
        None
    }
}

pub enum QueryData<'a> {
    None,
    Text(&'a str),
}

impl<'a> QueryData<'a> {
    pub fn get_text(&'a self) -> &'a str {
        if let Self::Text(s) = self {
            s
        } else {
            ""
        }
    }
}



#[derive(Debug)]
pub enum ParseError {
    InvalidLine,
    UnexpectedEnd(usize),
    InvalidData(usize),
    InvalidToken(usize),
    InvalidBase64(usize, DecodeError)
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLine => write!(f, "InvalidLine"),
            Self::UnexpectedEnd(i) => write!(f, "UnexpectedEnd({})", i),
            Self::InvalidData(i) => write!(f, "InvalidData({})", i),
            Self::InvalidToken(i) => write!(f, "InvalidToken({})", i),
            Self::InvalidBase64(i, e) => write!(f, "InvalidBase64({})", i),
        }
    }
}

impl error::Error for ParseError {

}



#[derive(Debug)]
pub enum OperationError {
    Invalid,
    PageOutOfBound(usize, usize),
    LineOutOfPage(usize, usize)
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "Invalid"),
            Self::PageOutOfBound(i, n) => write!(f, "PageOutOfBound:{}@{}", i, n),
            Self::LineOutOfPage(i, n) => write!(f, "LineOutOfPage:{}@{}", i, n),
        }
    }
}

impl error::Error for OperationError {

}