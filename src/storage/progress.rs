use std::io;
use std::io::{Read, Seek, SeekFrom};
//use std::io::{Write};
use indicatif::{ProgressBar, ProgressStyle};

/// Wrapper around a `Read` that reports the progress made.
///
/// Used to monitor slow IO readers
/// Unfortunately cannot use this with http client yet as it does not implement seek
pub struct ProgressReader<R: Read + Seek> {
    rdr: R,
    pb: ProgressBar,
}

/*pub fn copy_with_progress<R: ?Sized, W: ?Sized>(progress: &ProgressBar,
                                                reader: &mut R, writer: &mut W)
    -> io::Result<u64>
    where R: Read, W: Write
{
    let mut buf = [0; 16384];
    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        written += len as u64;
        progress.inc(len as u64);
    }
}
*/
impl<R: Read + Seek> ProgressReader<R> {
    pub fn new(mut rdr: R) -> io::Result<ProgressReader<R>> {
        let len = rdr.seek(SeekFrom::End(0))?;
        rdr.seek(SeekFrom::Start(0))?;
        let pb = ProgressBar::new(len);
        pb.set_style(ProgressStyle::default_bar()
                         .template("{bar:40.green/black} {bytes}/{total_bytes} ({eta})"));
        Ok(ProgressReader { rdr, pb })
    }
}

/*impl<R: Read + Seek> ProgressReader<R> {
    pub fn progress(&self) -> &ProgressBar {
        &self.pb
    }
}*/

impl<R: Read + Seek> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let rv = self.rdr.read(buf)?;
        self.pb.inc(rv as u64);
        Ok(rv)
    }
}
