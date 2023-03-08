use std::io::{Cursor, BufReader, Read, BufRead};
use std::fmt::Debug;

use async_trait::async_trait;
use futures::AsyncRead;

use crate::error::Error;
use crate::hdu::header::extension::bintable::BinTable;
use crate::hdu::DataBufRead;
use crate::hdu::data::image::DataOwnedIt;
use crate::hdu::data::image::InMemData;
use crate::hdu::data::image::DataBorrowed;

use crate::hdu::header::extension::Xtension;

use super::image::DataOwnedSt;
use super::{Access, DataAsyncBufRead};

impl<'a, R> DataBufRead<'a, BinTable> for Cursor<R>
where
    R: AsRef<[u8]> + Debug + Read + 'a
{
    type Data = DataBorrowed<'a, Self>;

    fn new_data_block(&'a mut self, ctx: &BinTable) -> Self::Data where Self: Sized {
        let num_bytes_read = ctx.get_num_bytes_data_block();

        let bytes = self.get_ref();
        let bytes = bytes.as_ref();

        let pos = self.position() as usize;
        let start_byte_pos = pos;
        let end_byte_pos = pos + num_bytes_read;

        let bytes = &bytes[start_byte_pos..end_byte_pos];

        let x_ptr = bytes as *const [u8] as *mut [u8];
        unsafe {
            let x_mut_ref = &mut *x_ptr;
    
            let (_, data, _) = x_mut_ref.align_to_mut::<u8>();
            let data = &data[..num_bytes_read];

            DataBorrowed {
                data: InMemData::U8(data),
                reader: self,
                num_bytes_read
            }
        }
    }

    fn consume_data_block(data: Self::Data, num_bytes_read: &mut usize) -> Result<&'a mut Self, Error> {
        let DataBorrowed {reader, num_bytes_read: num_bytes, ..} = data;
        *num_bytes_read = num_bytes;

        reader.set_position(reader.position() + num_bytes as u64);

        Ok(reader)
    }
}

impl<'a, R> DataBufRead<'a, BinTable> for BufReader<R>
where
    R: Read + Debug + 'a
{
    type Data = DataOwnedIt<'a, Self, u8>;

    fn new_data_block(&'a mut self, ctx: &BinTable) -> Self::Data {
        let num_bytes_to_read = ctx.get_num_bytes_data_block();
        DataOwnedIt::new(self, num_bytes_to_read)
    }

    fn consume_data_block(data: Self::Data, num_bytes_read: &mut usize) -> Result<&'a mut Self, Error> {
        let DataOwnedIt { reader, num_bytes_read: num_bytes_already_read, num_bytes_to_read, .. } = data;

        let remaining_bytes_to_read = num_bytes_to_read - num_bytes_already_read;
        <Self as DataBufRead<'_, BinTable>>::read_n_bytes_exact(reader, remaining_bytes_to_read)?;

        // All the data block have been read
        *num_bytes_read = num_bytes_to_read;

        Ok(reader)
    }
}

impl<'a, R> Access for DataOwnedIt<'a, R, u8>
where
    R: BufRead
{
    type Type = Self;

    fn get_data(&self) -> &Self::Type {
        self
    }

    fn get_data_mut(&mut self) -> &mut Self::Type {
        self
    }
}

#[async_trait]
impl<'a, R> DataAsyncBufRead<'a, BinTable> for futures::io::BufReader<R>
where
    R: AsyncRead + Debug + 'a + std::marker::Unpin + std::marker::Send,
{
    type Data = DataOwnedSt<'a, Self, u8>;

    fn new_data_block(&'a mut self, ctx: &BinTable) -> Self::Data {
        let num_bytes_to_read = ctx.get_num_bytes_data_block();
        DataOwnedSt::new(self, num_bytes_to_read)
    }

    async fn consume_data_block(data: Self::Data, num_bytes_read: &mut usize) -> Result<&'a mut Self, Error>
    where
        'a: 'async_trait
    {
        let DataOwnedSt { reader, num_bytes_to_read, num_bytes_read: num_bytes_already_read, .. } = data;

        let remaining_bytes_to_read = num_bytes_to_read - num_bytes_already_read;
        <Self as DataAsyncBufRead<'_, BinTable>>::read_n_bytes_exact(reader, remaining_bytes_to_read).await?;

        // All the data block have been read
        *num_bytes_read = num_bytes_to_read;

        Ok(reader)
    }
}