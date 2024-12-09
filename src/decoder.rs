use std::io::Read;

use anyhow::{bail, ensure, Result};
use flate2::read::ZlibDecoder;

use crate::grammar::{Chunk, Filter, ImageHeader, Png};

#[derive(Debug)]
pub struct Decoder<'a> {
    cursor: usize,
    data: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { cursor: 0, data }
    }

    pub fn decode(&mut self) -> Result<Png> {
        ensure!(
            self.read_slice(8)? == b"\x89PNG\r\n\x1A\n",
            "Expected signature.",
        );

        let chunks = self.parse_chunks()?;

        let mut chunks = chunks.into_iter();

        let Some(Chunk::ImageHeader(image_header)) = chunks.next() else {
            bail!("Expected image header chunk.");
        };

        let mut chunks = chunks.peekable();

        // There may be multiple image data chunks. If so, they shall appear
        // consecutively with no intervening chunks. The compressed stream is then
        // the concatenation of the contents of all image data chunks.
        let mut compressed_stream = Vec::new();

        // todo, how would you collect palettes if ColorType::Palette?
        // todo, how do you collect ancillary chunks?

        while let Some(&Chunk::ImageData(sub_data)) = chunks.peek() {
            compressed_stream.extend_from_slice(sub_data);
            chunks.next();
        }

        // todo!, what if ancillary chunks appear after the image data chunks?

        let mut zlib_decoder = ZlibDecoder::new(&compressed_stream[..]);
        let mut pixel_buffer = Vec::new();
        zlib_decoder.read_to_end(&mut pixel_buffer)?;

        // filter
        ensure!(
            image_header.filter_method == 0,
            "Only filter method 0 is defined in the standard."
        );

        let mut image_data = Vec::new();

        let bytes_per_row = image_header.num_bytes_per_pixel() * image_header.width as usize;

        for i in 0..image_header.height as usize {
            let mut row_start_idx = i * (1 + bytes_per_row);
            let filter_type = Filter::try_from(pixel_buffer[row_start_idx])?;
            row_start_idx += 1;
            let row = &pixel_buffer[row_start_idx..row_start_idx + bytes_per_row];

            match filter_type {
                Filter::None => {
                    // the best filter.
                }
                Filter::Sub => {
                    let mut image_row = Vec::new();
                    let mut prevs = vec![0; image_header.color_type.num_channels() as usize];

                    for i in 0..row.len() {
                        let prev_idx = i % 4;
                        let filtered = row[i].wrapping_add(prevs[prev_idx]);
                        image_row.push(filtered);
                        prevs[prev_idx] = filtered;
                    }

                    image_data.extend_from_slice(&image_row);
                }
                Filter::Up => todo!("What does the Up filter function look like?"),
                Filter::Average => todo!("What does the Average filter function look like?"),
                Filter::Paeth => todo!("What does the Paeth filter function look like?"),
            }
        }

        assert_eq!(
            image_data.len(),
            pixel_buffer.len() - image_header.height as usize
        );

        Ok(Png {
            width: image_header.width,
            height: image_header.height,
            color_type: image_header.color_type,
            image_data,
        })
    }

    fn parse_chunks(&mut self) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();

        loop {
            let length = self.read_u32()? as usize;

            let chunk = match self.read_slice(4)? {
                b"IHDR" => Chunk::ImageHeader(ImageHeader {
                    width: self.read_u32()?,
                    height: self.read_u32()?,
                    bit_depth: self.read_u8()?,
                    color_type: self.read_u8()?.try_into()?,
                    compression_method: self.read_u8()?,
                    filter_method: self.read_u8()?,
                    interlace_method: self.read_u8()? == 1,
                }),
                b"PLTE" => todo!("What does a palette look like?"),
                b"IDAT" => Chunk::ImageData(self.read_slice(length)?),
                b"IEND" => break,
                foreign => todo!(
                    "There may be some other chunks to handle at {}. Identifier: {:?}.",
                    self.cursor,
                    foreign
                ),
            };

            let _crc = self.read_u32()?;

            chunks.push(chunk);
        }

        Ok(chunks)
    }

    fn eof(&self, len: usize) -> Result<()> {
        let end = self.data.len();

        ensure!(
            self.cursor + len.saturating_sub(1) < self.data.len(),
            "Unexpected EOF. At {}, seek by {}, buffer size: {}.",
            self.cursor,
            len,
            end
        );

        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8> {
        self.eof(0)?;

        let b = self.data[self.cursor];
        self.cursor += 1;

        Ok(b)
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.eof(4)?;

        let slice = &self.data[self.cursor..self.cursor + 4];
        let n = u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]);

        self.cursor += 4;

        Ok(n)
    }

    fn read_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        self.eof(len)?;

        let slice = &self.data[self.cursor..self.cursor + len];
        self.cursor += len;

        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_potatoe() -> Result<()> {
        let content = std::fs::read("./tests/potatoe.png")?;
        let mut decoder = Decoder::new(&content);
        let png = decoder.decode()?;

        assert_eq!(png.height, 1158);
        assert_eq!(png.width, 2048);

        Ok(())
    }
}
