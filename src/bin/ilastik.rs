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

fn find_extents(mask : &Vec<u16>, height : usize, width : usize, start : usize, end : usize) -> Vec<(usize, usize)> {
    let mut extents :  Vec<(usize, usize)> = vec!();

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
        extents.push((w, h));
    }

    extents
}

/// Returns None
/// 
/// # Arguments
/// 

fn process_mask(mask : &Vec<u16>, raw: &Vec<f32>, height : usize, width : usize, nthreads : u32) {
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
    let mut extents : Vec<(usize, usize)> = vec!();
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

    for e in extents {
        if e.0 > max_w { max_w = e.0; }
        if e.1 > max_h { max_h = e.1; }
    }

    println!("Max extent (w, h) {}, {}", max_w, max_h);

}

fn main() {
    let args: Vec<_> = env::args().collect();
    
    if args.len() < 4 {
        println!("Usage: ilastic <path to raw tiff> <path to class tiff> <num threads>"); 
        process::exit(1);
    }

    let raw_tiff_path = Path::new(&args[1]);
    let obj_tiff_path = Path::new(&args[2]);
    let nthreads = &args[3].parse::<u32>().unwrap();
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
            process_mask(&img_res_obj, &img_res_raw, 1280, 1280, *nthreads);
        }

    } else {
        panic!("Wrong data type");
    }
}
