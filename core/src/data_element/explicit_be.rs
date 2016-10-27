//! Explicit VR Big Endian syntax transfer implementation.

use std::io::{Read, Write};
use std::fmt;
use attribute::ValueRepresentation;
use attribute::tag::Tag;
use byteorder::{ByteOrder, BigEndian};
use error::Result;
use super::decode::Decode;
use super::encode::Encode;
use std::marker::PhantomData;
use data_element::{DataElementHeader, SequenceItemHeader};

#[cfg(test)]
mod tests {
    use super::super::decode::Decode;
    use super::super::encode::Encode;
    use super::ExplicitVRBigEndianDecoder;
    use super::ExplicitVRBigEndianEncoder;
    use data_element::DataElementHeader;
    use attribute::ValueRepresentation;
    use std::io::{Read, Cursor, Seek, SeekFrom, Write};

    // manually crafting some DICOM data elements
    //  Tag: (0002,0002) Media Storage SOP Class UID
    //  VR: UI
    //  Length: 26
    //  Value: "1.2.840.10008.5.1.4.1.1.1" (with 1 padding '\0')
    // --
    //  Tag: (0002,0010) Transfer Syntax UID
    //  VR: UI
    //  Length: 20
    //  Value: "1.2.840.10008.1.2.1" (w 1 padding '\0') == ExplicitVRLittleEndian
    // --
    const RAW: &'static [u8; 62] = &[
        0x00, 0x02, 0x00, 0x02, 0x55, 0x49, 0x00, 0x1a, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e,
        0x31, 0x30, 0x30, 0x30, 0x38, 0x2e, 0x35, 0x2e, 0x31, 0x2e, 0x34, 0x2e, 0x31, 0x2e, 0x31, 0x2e,
        0x31, 0x00,

        0x00, 0x02, 0x00, 0x10, 0x55, 0x49, 0x00, 0x14, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e,
        0x31, 0x30, 0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32, 0x2e, 0x31, 0x00
    ];

    #[test]
    fn explicit_vr_be_works() {
        
        let reader = ExplicitVRBigEndianDecoder::default();
        let mut cursor = Cursor::new(RAW.as_ref());
        { // read first element
            let elem = reader.decode_header(&mut cursor).expect("should find an element");
            assert_eq!(elem.tag(), (2, 2));
            assert_eq!(elem.vr(), ValueRepresentation::UI);
            assert_eq!(elem.len(), 26);
            // read only half of the data
            let mut buffer: Vec<u8> = Vec::with_capacity(13);
            buffer.resize(13, 0);
            cursor.read_exact(buffer.as_mut_slice()).expect("should read it fine");
            assert_eq!(buffer.as_slice(), b"1.2.840.10008".as_ref());
        }
        // cursor should now be @ #21 (there is no automatic skipping)
        assert_eq!(cursor.seek(SeekFrom::Current(0)).unwrap(), 21);
        // cursor should now be @ #34 after skipping
        assert_eq!(cursor.seek(SeekFrom::Current(13)).unwrap(), 34);
        { // read second element
            let elem = reader.decode_header(&mut cursor).expect("should find an element");
            assert_eq!(elem.tag(), (2, 16));
            assert_eq!(elem.vr(), ValueRepresentation::UI);
            assert_eq!(elem.len(), 20);
            // read all data
            let mut buffer: Vec<u8> = Vec::with_capacity(20);
            buffer.resize(20, 0);
            cursor.read_exact(buffer.as_mut_slice()).expect("should read it fine");
            assert_eq!(buffer.as_slice(), b"1.2.840.10008.1.2.1\0".as_ref());
        }
    }

    #[test]
    fn encode_explicit_vr_be_works() {
        let mut buf = [0u8; 62];
        {
            let enc = ExplicitVRBigEndianEncoder::default();
            let mut writer = Cursor::new(&mut buf[..]);

            // encode first element
            let de = DataElementHeader {
                tag: (0x0002,0x0002),
                vr: ValueRepresentation::UI,
                len: 26,
            };
            enc.encode_element_header(de, &mut writer).expect("should write it fine");
            writer.write_all(b"1.2.840.10008.5.1.4.1.1.1\0".as_ref()).expect("should write the value fine");
        }
        assert_eq!(&buf[0..8], &RAW[0..8]);
        {
            let enc = ExplicitVRBigEndianEncoder::default();
            let mut writer = Cursor::new(&mut buf[34..]);

            // encode second element
            let de = DataElementHeader {
                tag: (0x0002,0x0010),
                vr: ValueRepresentation::UI,
                len: 20,
            };
            enc.encode_element_header(de, &mut writer).expect("should write it fine");
            writer.write_all(b"1.2.840.10008.1.2.1\0".as_ref()).expect("should write the value fine");
        }
        assert_eq!(&buf[34..42], &RAW[34..42]);

        assert_eq!(&buf[..], &RAW[..]);
    }
}

/// A data element decoder for the Explicit VR Big Endian transfer syntax.
pub struct ExplicitVRBigEndianDecoder<S: Read + ?Sized> {
    phantom: PhantomData<S>,
}

impl<S: Read + ?Sized> Default for ExplicitVRBigEndianDecoder<S> {
    fn default() -> ExplicitVRBigEndianDecoder<S> {
        ExplicitVRBigEndianDecoder{ phantom: PhantomData::default() }
    }
}

impl<S: Read + ?Sized> fmt::Debug for ExplicitVRBigEndianDecoder<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExplicitVRBigEndianDecoder")
    }
}

impl<'s, S: Read + ?Sized + 's> Decode for ExplicitVRBigEndianDecoder<S> {
    type Source = S;
    
    fn decode_header(&self, source: &mut Self::Source) -> Result<DataElementHeader> {
        let mut buf = [0u8; 4];
        try!(source.read_exact(&mut buf));
        // retrieve tag
        let group = BigEndian::read_u16(&buf[0..2]);
        let element = BigEndian::read_u16(&buf[2..4]);

        // retrieve explicit VR
        try!(source.read_exact(&mut buf[0..2]));
        let vr = ValueRepresentation::from_binary([buf[0], buf[1]]).unwrap_or(ValueRepresentation::UN);

        // retrieve data length
        let len = match vr {
            ValueRepresentation::OB | ValueRepresentation::OD |
            ValueRepresentation::OF | ValueRepresentation::OL |
            ValueRepresentation::OW | ValueRepresentation::SQ |
            ValueRepresentation::UC | ValueRepresentation::UR |
            ValueRepresentation::UT | ValueRepresentation::UN => {
                // read 2 reserved bytes, then 4 bytes for data length
                try!(source.read_exact(&mut buf[0..2]));
                try!(source.read_exact(&mut buf));
                BigEndian::read_u32(&buf)
            },
            _ => {
                // read 2 bytes for the data length
                try!(source.read_exact(&mut buf[0..2]));
                BigEndian::read_u16(&buf[0..2]) as u32
            }
        };

        Ok(DataElementHeader{ tag: (group, element), vr: vr, len: len })
    }

    fn decode_item_header(&self, source: &mut Self::Source) -> Result<SequenceItemHeader> {
        let mut buf = [0u8; 4];
        try!(source.read_exact(&mut buf));
        // retrieve tag
        let group = BigEndian::read_u16(&buf[0..2]);
        let element = BigEndian::read_u16(&buf[2..4]);

        try!(source.read_exact(&mut buf));
        let len = BigEndian::read_u32(&buf);

        SequenceItemHeader::new((group, element), len)
    }

    fn decode_us(&self, source: &mut Self::Source) -> Result<u16> {
        let mut buf = [0u8; 2];
        try!(source.read_exact(&mut buf[..]));
        Ok(BigEndian::read_u16(&buf[..]))
    }

    fn decode_ul(&self, source: &mut Self::Source) -> Result<u32> {
        let mut buf = [0u8; 4];
        try!(source.read_exact(&mut buf[..]));
        Ok(BigEndian::read_u32(&buf[..]))
    }

    fn decode_ss(&self, source: &mut Self::Source) -> Result<i16> {
        let mut buf = [0u8; 2];
        try!(source.read_exact(&mut buf[..]));
        Ok(BigEndian::read_i16(&buf[..]))
    }

    fn decode_sl(&self, source: &mut Self::Source) -> Result<i32> {
        let mut buf = [0u8; 4];
        try!(source.read_exact(&mut buf[..]));
        Ok(BigEndian::read_i32(&buf[..]))
    }
}

pub struct ExplicitVRBigEndianEncoder<W: Write + ?Sized> {
    phantom: PhantomData<W>
}

impl<W: Write + ?Sized> Default for ExplicitVRBigEndianEncoder<W> {
    fn default() -> ExplicitVRBigEndianEncoder<W> {
        ExplicitVRBigEndianEncoder{ phantom: PhantomData::default() }
    }
}

impl<W: Write + ?Sized> fmt::Debug for ExplicitVRBigEndianEncoder<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExplicitVRBigEndianEncoder")
    }
}

impl<W: Write + ?Sized> Encode for ExplicitVRBigEndianEncoder<W> {
    type Writer = W;

    fn encode_element_header(&self, de: DataElementHeader, to: &mut W) -> Result<()> {
        match de.vr {
            ValueRepresentation::OB | ValueRepresentation::OD |
            ValueRepresentation::OF | ValueRepresentation::OL |
            ValueRepresentation::OW | ValueRepresentation::SQ |
            ValueRepresentation::UC | ValueRepresentation::UR |
            ValueRepresentation::UT | ValueRepresentation::UN => {

                let mut buf = [0u8 ; 12];
                BigEndian::write_u16(&mut buf[0..], de.tag.group());
                BigEndian::write_u16(&mut buf[2..], de.tag.element());
                let vr_bytes = de.vr.to_bytes();
                buf[4] = vr_bytes[0];
                buf[5] = vr_bytes[1];
                // buf[6..8] is kept zero'd
                BigEndian::write_u32(&mut buf[8..], de.len);
                try!(to.write_all(&buf));

                Ok(())
            },
            _ => {
                let mut buf = [0u8; 8];
                BigEndian::write_u16(&mut buf[0..], de.tag.group());
                BigEndian::write_u16(&mut buf[2..], de.tag.element());
                let vr_bytes = de.vr.to_bytes();
                buf[4] = vr_bytes[0];
                buf[5] = vr_bytes[1];
                BigEndian::write_u16(&mut buf[6..], de.len as u16);
                try!(to.write_all(&buf));

                Ok(())
            }
        }
    }

    fn encode_item_header(&self, len: u32, to: &mut W) -> Result<()> {
        let mut buf = [0u8; 8];
        BigEndian::write_u16(&mut buf, 0xFFFE);
        BigEndian::write_u16(&mut buf, 0xE000);
        BigEndian::write_u32(&mut buf[4..], len);
        try!(to.write_all(&buf));
        Ok(())
    }

    fn encode_item_delimiter(&self, to: &mut W) -> Result<()> {
        let mut buf = [0u8; 8];
        BigEndian::write_u16(&mut buf, 0xFFFE);
        BigEndian::write_u16(&mut buf, 0xE00D);
        BigEndian::write_u32(&mut buf[4..], 0);
        try!(to.write_all(&buf));
        Ok(())
    }

    fn encode_sequence_delimiter(&self, to: &mut W) -> Result<()> {
        let mut buf = [0u8; 8];
        BigEndian::write_u16(&mut buf, 0xFFFE);
        BigEndian::write_u16(&mut buf, 0xE0DD);
        BigEndian::write_u32(&mut buf[4..], 0);
        try!(to.write_all(&buf));
        Ok(())
    }
}
