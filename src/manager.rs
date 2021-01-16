use std::fmt::Write as _;
use std::rc::Rc;
use std::cell::RefCell;
use async_std::io as aio;
use crate::operation::CommandSplit;
use crate::operation::Operation;
use crate::contents::Content;
use crate::contents::Page;
use crate::server::Handler;
use crate::server::Buf;


pub trait Canvas {
    
    fn draw(&mut self, page: &Page) -> aio::Result<()>;

    fn init(&mut self) -> aio::Result<()>;

    fn flush(&mut self) -> aio::Result<()>;

    fn clear(&mut self) -> aio::Result<()>;
}

pub struct Manager {
    inner: Rc<RefCell<ManagerInner>>
}

struct ManagerInner {
    content: Content,
    canvas: Box<dyn Canvas>,
    index: usize,
}

impl Manager {
    
    pub fn new(content: Content, canvas: Box<dyn Canvas>) -> aio::Result<Self> {
        let mut canvas = canvas;
        canvas.init()?;
        canvas.flush()?;
        let inner = ManagerInner {
            content,
            canvas,
            index: 0,
        };
        Ok(Manager {
            inner: Rc::new(RefCell::new(inner))
        })
    }
}


impl Handler for Manager {

    fn handle_network(&self, read: &mut Buf) -> aio::Result<Buf> {

        let resp = {
            let mut index = 0;
            let sp = CommandSplit::new(read.get(read.readable()), &mut index);
            let mut resp = String::with_capacity(256);
            let mut inner = self.inner.borrow_mut();
            for s in sp {
                match Operation::new(s) {
                    Ok(op) => {
                        match op.modify(&mut inner.content) {
                            Ok(query) => {
                                write!(&mut resp, "+{}\r\n", query.get_text()).unwrap();
                            }
    
                            Err(e) => {
                                write!(&mut resp, "-{}\r\n", e).unwrap();
                            }
                        }   
                    }
                    Err(e) => {
                        write!(&mut resp, "-{}\r\n", e).unwrap();
                    }
                }
            }
            read.skip(index);
            resp
        };

        {
            let mut inner = self.inner.borrow_mut();
            let content = &inner.content;
            if content.len() == 1 {
                let page = content.get(0).unwrap().clone();
                inner.canvas.draw(&page)?;
                inner.canvas.flush()?;
            }
        }

        Ok(Buf::from(resp.into_bytes()))      
    }

    fn handle_schedule(&self) -> aio::Result<()> {
        let mut inner = self.inner.borrow_mut();
        let content = &inner.content;
        let n = content.len();
        if n > 1 {
            let i = inner.index;
            let page = content.get(i).unwrap().clone();
            inner.canvas.draw(&page)?;
            inner.canvas.flush()?;
            inner.index = (i + 1) % n;
        }
        Ok(())
    }
}
