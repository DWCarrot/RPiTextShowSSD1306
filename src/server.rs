use std::slice;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::time::Duration;
use async_std::io as aio;
use async_std::io::Read as ARead;
use async_std::io::Write as AWrite;
use async_std::io::prelude::ReadExt as _;
use async_std::io::prelude::WriteExt as _;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::stream;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use futures::task::Context;
use futures::task::Poll;

pub struct Buf {
    buf: Box<[u8]>,
    read: usize,
    write: usize
}



impl From<Vec<u8>> for Buf {
    fn from(data: Vec<u8>) -> Self {
        let write = data.len();
        let buf = unsafe {
            let mut data = ManuallyDrop::new(data);
            let len = data.capacity();
            let ptr = data.as_mut_ptr();
            let slice = slice::from_raw_parts_mut(ptr, len);
            Box::from_raw(slice)
        };
        Buf {
            buf,
            read: 0,
            write
        }
    }
}

impl Buf {

    pub fn new(size: usize) -> Self {
        Buf {
            buf: vec![0u8; size].into_boxed_slice(),
            read: 0,
            write: 0
        }
    }

    pub fn readable(&self) -> usize {
        self.write - self.read
    }

    pub fn writeable(&self) -> usize {
        self.buf.len() - self.write
    }

    pub async fn write_from_reader<'a, R: ARead + Unpin>(&mut self, src: &'a mut R) -> aio::Result<usize> {
        let slice = unsafe { self.buf.get_unchecked_mut(self.write..) };
        let len = src.read(slice).await?;
        self.write += len;
        Ok(len)
    }

    pub async fn read_to_writer<'a, W: AWrite + Unpin>(&mut self, tgt: &'a mut W) -> aio::Result<usize> {
        let slice = unsafe { self.buf.get_unchecked_mut(self.read .. self.write) };
        let len = tgt.write(slice).await?;
        self.read += len;
        Ok(len)
    }

    pub async fn read_all_to_writer<'a, W: AWrite + Unpin>(&mut self, tgt: &'a mut W) -> aio::Result<usize> {
        let slice = unsafe { self.buf.get_unchecked_mut(self.read .. self.write) };
        tgt.write_all(slice).await?;
        let len = self.write - self.read;
        self.read = self.write;
        Ok(len)
    }

    pub fn get<'a>(&'a self, len: usize) -> &'a [u8] {
        let i = self.read;
        let j = std::cmp::min(i + len, self.write);
        unsafe{ self.buf.get_unchecked(i..j) }
    }

    pub fn skip(&mut self, len: usize) {
        self.read = std::cmp::min(self.read + len, self.write);
    }

    pub fn flip(&mut self) -> usize {
        let count = self.write - self.read;
        if count > 0 {
            unsafe {
                let src = self.buf.as_ptr().offset(self.read as isize);
                let dst = self.buf.as_mut_ptr();
                std::ptr::copy(src, dst, count);
            }
        }
        self.write = count;
        self.read = 0;
        count
    }

    pub fn reset(&mut self) {
        self.read = 0;
        self.write = 0;
    }
}

pub trait Handler {
    fn handle_network(&self, read: &mut Buf) -> aio::Result<Buf>;
    fn handle_schedule(&self) -> aio::Result<()>;
}

pub struct Server<H: Handler> {
    n_worker: usize,
    buf_size: usize,
    handler: H,
    interval: Duration,
}

impl<H: Handler> Server<H> {

    pub fn new(handler: H, interval: Duration, buf_size: usize, n_worker: usize) -> Self {
        Server {
            n_worker: n_worker + 1,
            buf_size,
            handler,
            interval
        }
    }

    pub async fn start_server(&self, addr: impl ToSocketAddrs) -> aio::Result<()> {

        let scheduler = stream::interval(self.interval);
        let listener = TcpListener::bind(addr).await?;
        CombinedStream::new(scheduler, listener.incoming())
            .for_each_concurrent(Some(self.n_worker), |stream| async move {
                match stream {
                    CombinedStreamOutput::First(_) => {
                        if let Err(e) = self.schedule_job().await {
                            eprintln!("{}", e)
                        }
                    },
                    CombinedStreamOutput::Second(stream) => {
                        if let Err(e) = self.process(stream).await {
                            eprintln!("{}", e)
                        }
                    }
                }
            })
            .await;
        Ok(())
    }

    async fn process(&self, stream: aio::Result<TcpStream>) -> aio::Result<()> {
        let mut stream = stream?;
        let mut buf = Buf::new(self.buf_size);
        let handler = &self.handler;
        loop {
            let c = buf.write_from_reader(&mut stream).await?;
            if c == 0 {
                break;
            }
            let mut response = handler.handle_network(&mut buf)?;
            buf.flip();
            response.read_all_to_writer(&mut stream).await?;         
        }
        Ok(())
    }

    async fn schedule_job(&self) -> aio::Result<()> {
        self.handler.handle_schedule()?;
        Ok(())
    }
}


pub enum CombinedStreamOutput<T1, T2> {
    First(T1),
    Second(T2)
}

pub struct CombinedStream<S1, S2> {
    s1: S1,
    finished1: bool,
    s2: S2,
    finished2: bool,
}

impl<T1, S1, T2, S2> CombinedStream<S1, S2>
where
    S1: Stream<Item=T1> + Unpin,
    S2: Stream<Item=T2> + Unpin
{
    pub fn new(s1: S1, s2: S2) -> Self {
        CombinedStream {
            s1,
            finished1: false,
            s2,
            finished2: false
        }
    }
}

impl<T1, S1, T2, S2> Stream for CombinedStream<S1, S2> 
where
    S1: Stream<Item=T1> + Unpin,
    S2: Stream<Item=T2> + Unpin
{
    type Item = CombinedStreamOutput<T1, T2>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self;
        let mut ready = false;
        if !this.finished1 {
            if let Poll::Ready(it) = Stream::poll_next(Pin::new(&mut this.s1), cx) {
                ready = true;
                if let Some(v) = it {
                    return Poll::Ready(Some(CombinedStreamOutput::First(v)));
                } else {
                    this.finished1 = true;
                }     
            }      
        }
        if !this.finished2 {
            if let Poll::Ready(it) = Stream::poll_next(Pin::new(&mut this.s2), cx) {
                ready = true;
                if let Some(v) = it {
                    return Poll::Ready(Some(CombinedStreamOutput::Second(v)));
                } else {
                    this.finished2 = true;
                }            
            }    
        }
        if ready {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}