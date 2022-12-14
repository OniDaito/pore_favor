/// A small program that extracts pores from a large tiff
/// using an object identity file made with ilastik.
/// 
/// Author: Benjamin Blundell
/// Email: k1803390@kcl.ac.uk
/// 

extern crate fitrs;
extern crate tiff;

use std::env;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use fitrs::{Fits, Hdu};
use std::path::Path;
use std::error::Error;
use pbr::ProgressBar;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::ColorType;
use scoped_threadpool::Pool;
use std::sync::mpsc::channel;
use std::process;
use std::f32::consts::PI;

pub enum Direction {
    Right,
    Down,
    Left
 }

pub fn aug_img(img : &Vec<Vec<f32>>, dir : Direction) -> Vec<Vec<f32>> {
    let mut new_img : Vec<Vec<f32>> = vec!();
    let height = img.len() as i32;
    let width = img[0].len() as i32;
    let mut rm : [[i32; 2]; 2] = [[0, -1],[1, 0]];

    for y in 0..img.len() {
        let mut row : Vec<f32> = vec!();
        for x in 0..img[0].len() {
            row.push(0.0);
        }
        new_img.push(row);
    }
    match dir { // *self has type Direction
        Direction::Right => {
            rm = [[0, -1],[1, 0]];
        },
        Direction::Down =>{
            rm = [[-1, 0],[0, -1]];
        },
        Direction::Left => {
            rm = [[0, 1],[-1, 0]];
        },
    }

    for y in 0..img.len() {
        for x in 0..img[0].len() {

            match dir { // *self has type Direction
                Direction::Right => {
                    let nx = (width - 1) + (x as i32 * rm[0][0] + y as i32 * rm[0][1]);
                    let ny = x as i32 * rm[1][0] + y as i32 * rm[1][1];
                    new_img[y][x] = img[ny as usize][nx as usize];
                },
                Direction::Down =>{
                    let nx = (width - 1) + (x as i32 * rm[0][0] + y as i32 * rm[0][1]);
                    let ny = (height - 1) + (x as i32 * rm[1][0] + y as i32 * rm[1][1]);
                    new_img[y][x] = img[ny as usize][nx as usize];
                },
                Direction::Left => {
                    let nx = x as i32 * rm[0][0] + y as i32 * rm[0][1];
                    let ny = (height - 1) + (x as i32 * rm[1][0] + y as i32 * rm[1][1]);
                    new_img[y][x] = img[ny as usize][nx as usize];
                },
            }
        }
    }

    new_img
}


 // Perform a gauss blur
 pub fn gauss_blur(img : &Vec<Vec<f32>>, gauss : f32 ) -> Vec<Vec<f32>> {
    // http://blog.ivank.net/fastest-gaussian-blur.html
    let rs = (gauss * 2.57).ceil() as usize;
    let height = img.len();
    let width = img[0].len();
    
    // New temp image
    let mut img_blurred : Vec<Vec<f32>> = vec![];

    for y in 0..height {
        let mut row : Vec<f32> = vec!();
        for x in 0..width {
            row.push(0f32);
        }
        img_blurred.push(row);
    }

    for h in 0..height {
        for w in 0..width {
            let mut val : f32 = 0.0;
            let mut wsum : f32 = 0.0;

            for i in 0..(rs*2+1) {
                let iy : f32 = (h as f32 ) - (rs as f32) + (i as f32);

                for j in 0..(rs*2+1) {
                    let ix : f32 = (w as f32 ) - (rs as f32) + (j as f32);

                    let x = ((width - 1) as f32).min(0f32.max(ix)) as usize;
                    let y = ((height -1) as f32).min(0f32.max(iy)) as usize;
                    let dsq = (ix - w as f32) * (ix - w as f32) + (iy - h as f32) * (iy - h as f32);
                    let wght = (-dsq / (2.0*gauss*gauss)).exp() / (PI * 2.0 * gauss * gauss);
                    val += img[y][x] * wght;
                    wsum += wght;
                }
            }
            img_blurred[h][w] = val / wsum;
        }
    }
    img_blurred
}

/// Returns None
/// Save a fits image
/// # Arguments
/// 
/// * `img` - A Vec of Vectors of f32 - the pixels
/// * `height` - usize for the image height 
/// * `width` - the width of the image as usize 
/// * `filename` - A String - the filename to save
///
fn save_final_fits(img : &Vec<Vec<f32>>, height : usize, width : usize, filename : &String) {
    let mut data : Vec<f32> = (0..height)
        .map(|i| (0..width).map(
               move |j| (i + j) as f32)).flatten().collect();

    for _y in 0..height {
        for _x in 0..width {
            let idx : usize = (_y * width +_x ) as usize; 
            data[idx] = img[_x as usize][(height - _y - 1) as usize];
            // / intensity * MULTFAC;
        }
    }

    let mut primary_hdu = 
        Hdu::new(&[width as usize , height as usize], data);
    // Insert values in header
    primary_hdu.insert("NORMALISATION", "NONE");
    primary_hdu.insert("WIDTH", width as i32);
    primary_hdu.insert("HEIGHT", height as i32);
    Fits::create(filename, primary_hdu).expect("Failed to create");  
}

/// Returns None
/// 
/// # Arguments
/// 
///

fn find_extents(mask : &Vec<u16>, height : usize, width : usize, start : usize, end : usize) -> Vec<(usize, usize, usize, usize, usize)> {
    let mut extents :  Vec<(usize, usize, usize, usize, usize)> = vec!();

    for _i in start..end {
        // Now create the image we shall save as a fits
        let new_image : Vec<Vec<f32>> = vec!();

        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;

        for y in 0..height {
            for x in 0..width {
                let pos = y * width + x;

                if mask[pos] as usize == _i {
                    if x < min_x { min_x = x; }
                    if x > max_x { max_x = x; }
                    if y < min_y { min_y = y; }
                    if y > max_y { max_y = y; }
                }
            }
        }
        
        let w = max_x - min_x;
        let h = max_y - min_y;
        extents.push((w, h, min_x, min_y, _i));
    }

    extents
}


/// Returns None
/// 
/// # Arguments
/// 
///

fn cut_image(raw_image : &Vec<f32>, image_size : usize, raw_width : usize, extents : &Vec<(usize, usize, usize, usize, usize)>, start : usize, end : usize, gauss: f32)  -> usize {
    let mut count = start * 4;

    for _i in start..end {
        let idx = _i;
        // Now create the image we shall save as a fits
        let w = extents[idx].0;
        let h = extents[idx].1;
        let xstart = extents[idx].2;
        let ystart = extents[idx].3;
        let ridx = extents[idx].4;
        let mut new_image : Vec<Vec<f32>> = vec!();
        
        // Allocate 0s
        for _y in 0..image_size {
            let mut row : Vec<f32> = vec!();

            for _x in 0..image_size {
                row.push(0.0);
            }

            new_image.push(row);
        }

        for y in 0..h {

            for x in 0..w {
                let raw_pos = (y + ystart) * raw_width + x + xstart;

                if raw_pos < raw_image.len() {
                    new_image[y][x] = raw_image[raw_pos];
                }
            }
        }
        
        // Gaussian blur on top
        if gauss != 0.0 {
            new_image = gauss_blur(&new_image, gauss);
        }

        let mut fidx = format!("image_{:06}.fits", count as usize);
        println!("New Image {}, {}, {}, {}, {}", ridx, xstart, ystart, w, h);
        save_final_fits(&new_image, image_size, image_size, &fidx);
        count = count + 1;

        // now Aug 3 times
        let left = aug_img(&new_image, Direction::Left);
        fidx = format!("image_{:06}.fits", count as usize);
        count = count + 1;
        save_final_fits(&left, image_size, image_size, &fidx);

        let right = aug_img(&new_image, Direction::Right);
        fidx = format!("image_{:06}.fits", count as usize);
        count = count + 1;
        save_final_fits(&right, image_size, image_size, &fidx);

        let down = aug_img(&new_image, Direction::Down);
        fidx = format!("image_{:06}.fits", count as usize);
        count = count + 1;
        save_final_fits(&down, image_size, image_size, &fidx);

    }
    end - start
}


/// Returns None
/// 
/// # Arguments
/// 

fn process_mask(mask : &Vec<u16>, raw: &Vec<f32>, height : usize, width : usize, nthreads : u32, gauss: f32) {
    let mut total_objs : u32 = 0;
    
    for val in mask {
        if *val > total_objs as u16 {
            total_objs = *val as u32;
        }
    }

    println!("Number of objects {}", total_objs);

    let mut progress : i32 = 0;
    let mut pool = Pool::new(nthreads);
    let truns = (total_objs / nthreads) as u32;
    let spare = (total_objs % nthreads) as u32;
    let mut extents : Vec<(usize, usize, usize, usize, usize)> = vec!();
    let (tx, rx) = channel();

    // Break the range into groups for each thread
    pool.scoped(|scoped| {

        for _t in 0..nthreads {
            let tx = tx.clone();
            let mut start : usize = (_t * truns) as usize;
            if start == 0 { start = 1; }
            let mut end = ((_t + 1)  * truns) as usize;
            if _t == nthreads - 1 { end = end + (spare as usize); }
           
            scoped.execute( move || { 
                ///println!("Start {} - end {}", start, end);
                let textents = find_extents(mask, height, width, start, end);
                tx.send(textents).unwrap();
            });
        }
    });

    while progress < nthreads as i32 {
        match rx.try_recv() {
            Ok(_a) => {
                progress = progress + 1;
                println!("Progress");
                for e in _a { extents.push(e); }
            }, Err(_e) => {}
        }
    }

    let mut max_h : usize = 0;
    let mut max_w : usize = 0;

    for e in &extents {
        if e.0 > max_w { max_w = e.0; }
        if e.1 > max_h { max_h = e.1; }
    }

    println!("Max extent (w, h) {}, {}", max_w, max_h);

    let mut max_dim = max_w;
    if max_h > max_w {
        max_dim = max_h;
    }

    let (tx2, rx2) = channel();
    // Now cut up the image into smaller images taking the max extent
    pool.scoped(|scoped| {

        for _t in 0..nthreads {
            let tx2 = tx2.clone();
            let extents = extents.clone();
            let mut start : usize = (_t * truns) as usize;
            let mut end = ((_t + 1) * truns) as usize;
            if _t == nthreads - 1 { end = end + (spare as usize) - 1; }
           
            scoped.execute( move || { 
                let done = cut_image(raw, max_dim, width, &extents, start, end, gauss);
                tx2.send(done).unwrap();
            });
        }
    });

    while progress < nthreads as i32 {
        match rx2.try_recv() {
            Ok(_a) => {
                progress = progress + 1;
                println!("Progress");
            }, Err(_e) => {}
        }
    }


}

fn main() {
    let args: Vec<_> = env::args().collect();
    
    if args.len() < 4 {
        println!("Usage: ilastic <path to raw tiff> <path to class tiff> <num threads> <optional: gauss blur>"); 
        process::exit(1);
    }

    let raw_tiff_path = Path::new(&args[1]);
    let obj_tiff_path = Path::new(&args[2]);
    let nthreads = &args[3].parse::<u32>().unwrap();
    let mut gauss:f32 = 0.0;

    if args.len() == 5 {
        gauss = args[4].parse::<f32>().unwrap();
    }

    let img_file_raw = File::open(raw_tiff_path).expect("Cannot find test image!");
    let mut decoder_raw = Decoder::new(img_file_raw).expect("Cannot create decoder");

    assert_eq!(decoder_raw.colortype().unwrap(), ColorType::Gray(32));
    assert_eq!(decoder_raw.dimensions().unwrap(), (1280, 1280));
    
    if let DecodingResult::F32(img_res_raw) = decoder_raw.read_image().unwrap() {
        println!("Raw Image Loaded.");
        let img_file_obj = File::open(obj_tiff_path).expect("Cannot find test image!");
        let mut decoder_obj = Decoder::new(img_file_obj).expect("Cannot create decoder");
        assert_eq!(decoder_obj.colortype().unwrap(), ColorType::Gray(16));
        assert_eq!(decoder_obj.dimensions().unwrap(), (1280, 1280));

        if let DecodingResult::U16(img_res_obj) = decoder_obj.read_image().unwrap() {
            println!("Obj Image Loaded.");
            process_mask(&img_res_obj, &img_res_raw, 1280, 1280, *nthreads, gauss);
        }

    } else {
        panic!("Wrong data type");
    }
}
