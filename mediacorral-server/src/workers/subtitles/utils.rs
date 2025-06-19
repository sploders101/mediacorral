use image::RgbaImage;

pub fn crop_image(image: &RgbaImage) -> RgbaImage {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel = image.get_pixel(x, y);
            if pixel.0[3] > 0 {
                match bounds {
                    Some((ref mut x1, _y1, ref mut x2, ref mut y2)) => {
                        if *x1 > x {
                            *x1 = x;
                        }
                        if *x2 < x {
                            *x2 = x;
                        }
                        // y1 not needed due to scanning semantics
                        if *y2 < y {
                            *y2 = y;
                        }
                    }
                    None => {
                        bounds = Some((x, y, x, y));
                    }
                }
            }
        }
    }
    match bounds {
        None => {
            return RgbaImage::new(0, 0);
        }
        Some((x1, y1, x2, y2)) => {
            let mut new_image = RgbaImage::new(x2 + 1 - x1, y2 + 1 - y1);
            for (new_y, y) in (y1..=y2).enumerate() {
                for (new_x, x) in (x1..=x2).enumerate() {
                    new_image.put_pixel(new_x as _, new_y as _, image.get_pixel(x, y).clone());
                }
            }
            return new_image;
        }
    }
}
