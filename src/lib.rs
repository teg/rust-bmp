use std::io::{File, Open, Read, Append, ReadWrite, IoResult,
    SeekSet, SeekCur};

#[deriving(Show, PartialEq)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

pub mod consts;

#[deriving(Show)]
struct BmpId {
    magic1: u8,
    magic2: u8
}

impl BmpId {
    pub fn new() -> BmpId {
        BmpId {
            magic1: 0x42 /* 'B' */,
            magic2: 0x4D /* 'M' */
        }
    }
}

#[deriving(Show)]
struct BmpHeader {
    file_size: u32,
    creator1: u16,
    creator2: u16,
    pixel_offset: u32
}

impl BmpHeader {
    pub fn new(width: u32, height: u32) -> BmpHeader {
        BmpHeader {
            file_size: width * height * 4 /* bytes per pixel */ + 54 /* Header size */,
            creator1: 0 /* Unused */,
            creator2: 0 /* Unused */,
            pixel_offset: 54
        }
    }
}

#[deriving(Show)]
struct BmpDibHeader {
    header_size: u32,
    width: i32,
    height: i32,
    num_planes: u16,
    bits_per_pixel: u16,
    compress_type: u32,
    data_size: u32,
    hres: i32,
    vres: i32,
    num_colors: u32,
    num_imp_colors: u32,
}

impl BmpDibHeader {
    pub fn new(width: i32, height: i32) -> BmpDibHeader {
        let row_size = ((24.0 * width as f32 + 31.0) / 32.0).floor() as u32 * 4;
        let pixel_array_size = row_size * height as u32;

        BmpDibHeader {
            header_size: 40,
            width: width,
            height: height,
            num_planes: 1,
            bits_per_pixel: 24,
            compress_type: 0,
            data_size: pixel_array_size,
            hres: 0x100,
            vres: 0x100,
            num_colors: 0,
            num_imp_colors: 0
        }
    }
}

pub struct Image {
    magic: BmpId,
    header: BmpHeader,
    dib_header: BmpDibHeader,
    width: i32,
    height: i32,
    padding: i32,
    padding_data: [u8, .. 4],
    data: Option<Vec<Pixel>>
}

impl Image {
    pub fn new(width: i32, height: i32) -> Image {
        let mut data = Vec::with_capacity((width * height) as uint);
        for _ in range(0, width * height) {
            data.push(Pixel {r: 0, g: 0, b: 0});
        }
        Image {
            magic: BmpId::new(),
            header: BmpHeader::new(width as u32, height as u32),
            dib_header: BmpDibHeader::new(width, height),
            width: width,
            height: height,
            padding: width % 4,
            padding_data: [0, 0, 0, 0],
            data: Some(data)
        }
    }

    pub fn set_pixel(&mut self, x: uint, y: uint, val: Pixel) {
        if x < self.width as uint && y < self.height as uint {
            let data = match self.data {
                Some(ref mut data) => data.as_mut_slice(),
                None => fail!("Image has no data")
            };
            data[y * (self.width as uint) + x] = val;
        } else {
            fail!("Index out of bounds: ({}, {})", x, y);
        }
    }

    pub fn get_pixel(&self, x: uint, y: uint) -> Pixel {
        if x < self.width as uint && y < self.height as uint {
            match self.data {
                Some(ref data) => data[y * (self.width as uint) + x],
                None => fail!("Image has no data")
            }
        } else {
            fail!("Index out of bounds: ({}, {})", x, y);
        }
    }

    fn write_header(&self, name: &str) {
        let mut f = File::create(&Path::new(name));
        let id = self.magic;
        access(f.write([id.magic1, id.magic2]));

        let header = self.header;
        access(f.write_le_u32(header.file_size));
        access(f.write_le_u16(header.creator1));
        access(f.write_le_u16(header.creator2));
        access(f.write_le_u32(header.pixel_offset));

        let dib_header = self.dib_header;
        access(f.write_le_u32(dib_header.header_size));
        access(f.write_le_i32(dib_header.width));
        access(f.write_le_i32(dib_header.height));
        access(f.write_le_u16(dib_header.num_planes));
        access(f.write_le_u16(dib_header.bits_per_pixel));
        access(f.write_le_u32(dib_header.compress_type));
        access(f.write_le_u32(dib_header.data_size));
        access(f.write_le_i32(dib_header.hres));
        access(f.write_le_i32(dib_header.vres));
        access(f.write_le_u32(dib_header.num_colors));
        access(f.write_le_u32(dib_header.num_imp_colors));
    }

    pub fn save(&self, name: &str) {
        self.write_header(name);

        let mut file = match File::open_mode(&Path::new(name), Append, ReadWrite) {
            Ok(f) => f,
            Err(e) => fail!("File error: {}", e),
        };

        match self.data {
            Some(ref data) => {
                for y in range(0, self.height) {
                    for x in range(0, self.width) {
                        let index: uint = (y * self.width + x) as uint;
                        let p = data[index as uint];
                        access(file.write([p.b, p.g, p.r]));
                    }
                    let p = self.padding_data.slice(0, self.padding as uint);
                    access(file.write(p));
                }
            },
            None => fail!("Image has no data")
        }
    }

    fn read_bmp_id(f: &mut File) -> Option<BmpId> {
        match f.eof() {
            false =>
                Some(BmpId {
                    magic1: access(f.read_byte()),
                    magic2: access(f.read_byte())
                }),
            true => None
        }
    }

    fn read_bmp_header(f: &mut File) -> Option<BmpHeader> {
        match f.eof() {
            false =>
                Some(BmpHeader {
                    file_size: access(f.read_le_u32()),
                    creator1: access(f.read_le_u16()),
                    creator2: access(f.read_le_u16()),
                    pixel_offset: access(f.read_le_u32())
                }),
            true => None
        }
    }

    fn read_bmp_dib_header(f: &mut File) -> Option<BmpDibHeader> {
        match f.eof() {
            false =>
                Some(BmpDibHeader {
                    header_size: access(f.read_le_u32()),
                    width: access(f.read_le_i32()),
                    height: access(f.read_le_i32()),
                    num_planes: access(f.read_le_u16()),
                    bits_per_pixel: access(f.read_le_u16()),
                    compress_type: access(f.read_le_u32()),
                    data_size: access(f.read_le_u32()),
                    hres: access(f.read_le_i32()),
                    vres: access(f.read_le_i32()),
                    num_colors: access(f.read_le_u32()),
                    num_imp_colors: access(f.read_le_u32()),
                }),
            true => None
        }
    }

    fn read_pixel(f: &mut File) -> Pixel {
        let [b, g, r] = [
            access(f.read_byte()),
            access(f.read_byte()),
            access(f.read_byte())
        ];
        Pixel{r: r, g: g, b: b}
    }

    fn read_image_data(f: &mut File, dh: BmpDibHeader, offset: u32, padding: i64) -> Option<Vec<Pixel>> {
        let data_size = ((24.0 * dh.width as f32 + 31.0) / 32.0).floor() as u32
            * 4 * dh.height as u32;

        if data_size == dh.data_size {
            let mut data = Vec::new();
            // seek until data
            access(f.seek(offset as i64, SeekSet));
            // read pixels until padding
            for _ in range(0, dh.height) {
                for _ in range(0, dh.width) {
                   data.push(Image::read_pixel(f));
                }
                // seek padding
                access(f.seek(padding, SeekCur));
            }
            Some(data)
        } else {
            None
        }
    }

    pub fn open(name: &str) -> Image {
        let mut f = match File::open_mode(&Path::new(name), Open, Read) {
            Ok(f) => f,
            Err(e) => fail!("File error: {}", e),
        };

        let id = match Image::read_bmp_id(&mut f) {
            Some(id) => id,
            None => fail!("File is not a bitmap")
        };
        assert_eq!(id.magic1, 0x42);
        assert_eq!(id.magic2, 0x4D);

        let header = match Image::read_bmp_header(&mut f) {
            Some(header) => header,
            None => fail!("Header of bitmap is not valid")
        };

        let dib_header = match Image::read_bmp_dib_header(&mut f) {
            Some(dib_header) => dib_header,
            None => fail!("DIB header of bitmap is not valid")
        };

        let padding = dib_header.width % 4;
        Image {
            magic: id,
            header: header,
            dib_header: dib_header,
            width: dib_header.width,
            height: dib_header.height,
            padding: padding,
            padding_data: [0, 0, 0, 0],
            data: Image::read_image_data(&mut f, dib_header, header.pixel_offset, padding as i64)
        }
    }
}

fn access<T>(res: IoResult<T>) -> T {
    match res {
        Err(e) => fail!("File error: {}", e),
        Ok(r) => r
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use std::io::{File, SeekSet};
    use std::io::fs::PathExtensions;

    use BmpId;
    use BmpHeader;
    use BmpDibHeader;
    use Image;
    use Pixel;
    use consts::{RED, LIME, BLUE, WHITE};

    #[test]
    fn size_of_bmp_header_is_54_bytes() {
        let bmp_magic_size = size_of::<BmpId>();
        let bmp_header_size = size_of::<BmpHeader>();
        let bmp_bip_header_size = size_of::<BmpDibHeader>();

        assert_eq!(2,  bmp_magic_size);
        assert_eq!(12, bmp_header_size);
        assert_eq!(40, bmp_bip_header_size);
    }

    #[test]
    fn size_of_4pixel_bmp_image_is_70_bytes() {
        let path_wd = Path::new("src/test/rgbw.bmp");
        let size = path_wd.lstat().unwrap().size as i32;
        assert_eq!(70, size);
    }

    fn verify_test_bmp_image(img: Image) {
        let header = img.header;
        assert_eq!(70, header.file_size);
        assert_eq!(0,  header.creator1);
        assert_eq!(0,  header.creator2);

        let dib_header = img.dib_header;
        assert_eq!(54, header.pixel_offset);
        assert_eq!(40,    dib_header.header_size);
        assert_eq!(2,     dib_header.width);
        assert_eq!(2,     dib_header.height);
        assert_eq!(1,     dib_header.num_planes);
        assert_eq!(24,    dib_header.bits_per_pixel);
        assert_eq!(0,     dib_header.compress_type);
        assert_eq!(16,    dib_header.data_size);
        assert_eq!(0x100, dib_header.hres);
        assert_eq!(0x100, dib_header.vres);
        assert_eq!(0,     dib_header.num_colors);
        assert_eq!(0,     dib_header.num_imp_colors);

        assert_eq!(2, img.padding);
    }

    #[test]
    fn can_read_bmp_image() {
        let bmp_img = Image::open("src/test/rgbw.bmp");
        verify_test_bmp_image(bmp_img);
    }

    #[test]
    fn can_read_image_data() {
        let mut f = match File::open(&Path::new("src/test/rgbw.bmp"), ) {
            Ok(file) => file,
            Err(e) => fail!("File error: {}", e)
        };
        assert_eq!(0x42, f.read_byte().unwrap());
        assert_eq!(0x4D, f.read_byte().unwrap());

        match f.seek(54, SeekSet) {
            Ok(_) => (),
            Err(e) => fail!("Seek error: {}", e)
        }

        let pixel = Pixel {
            r: f.read_byte().unwrap(),
            g: f.read_byte().unwrap(),
            b: f.read_byte().unwrap()
        };

        assert_eq!(pixel, RED);
    }

    #[test]
    fn can_read_entire_bmp_image() {
        let bmp_img = Image::open("src/test/rgbw.bmp");
        assert!(None != bmp_img.data);

        assert_eq!(bmp_img.get_pixel(0, 0), BLUE);
        assert_eq!(bmp_img.get_pixel(1, 0), WHITE);
        assert_eq!(bmp_img.get_pixel(0, 1), RED);
        assert_eq!(bmp_img.get_pixel(1, 1), LIME);
    }

    #[test]
    fn can_create_bmp_file() {
        let mut bmp = Image::new(2, 2);
        bmp.set_pixel(0, 0, RED);
        bmp.set_pixel(1, 0, WHITE);
        bmp.set_pixel(0, 1, BLUE);
        bmp.set_pixel(1, 1, LIME);
        bmp.save("src/test/rgbw_test.bmp");

        let bmp_img = Image::open("src/test/rgbw_test.bmp");
        assert_eq!(bmp_img.get_pixel(0, 0), RED);
        assert_eq!(bmp_img.get_pixel(1, 0), WHITE);
        assert_eq!(bmp_img.get_pixel(0, 1), BLUE);
        assert_eq!(bmp_img.get_pixel(1, 1), LIME);

        verify_test_bmp_image(bmp_img);
    }

    #[test]
    fn changing_pixels_does_not_push_image_data() {
        let mut img = Image::new(2, 1);
        img.set_pixel(1, 0, WHITE);
        img.set_pixel(0, 0, WHITE);

        assert_eq!(img.get_pixel(0, 0), WHITE);
        assert_eq!(img.get_pixel(1, 0), WHITE);
    }
}
