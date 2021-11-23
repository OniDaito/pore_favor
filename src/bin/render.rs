/// A small program that reads a CSV file containing the
/// nuclear pore data.
/// 
/// Author: Benjamin Blundell
/// Email: k1803390@kcl.ac.uk

extern crate rand;
extern crate image;
extern crate nalgebra as na;
extern crate probability;
extern crate scoped_threadpool;
extern crate fitrs;
extern crate rand_distr;
extern crate ndarray;
extern crate csv;

use std::env;
use std::fmt;
use rand::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use fitrs::{Fits, Hdu};
use rand_distr::{Normal, Distribution};
use std::process;
use std::path::Path;
use std::error::Error;
use rand::distributions::Uniform;
use rand::Rng;
use scoped_threadpool::Pool;
use std::sync::mpsc::channel;
use pbr::ProgressBar;
use ndarray::{Slice, SliceInfo, s, Array1};

static WIDTH : u32 = 1280;
static HEIGHT : u32 = 1280;
static SHRINK : f32 = 0.95;

#[derive(Copy, Clone)]
pub struct Point {
    x : f32,
    y : f32
}

/// Returns two f32 numbers - the extents in X and Y.
/// Go through all the models and find the extents. This gives
/// us a global scale, we can use in the rendering.
/// 
/// # Arguments
/// 
/// * `models` - A Vector of Vectors of Point
/// 

fn find_extents ( models : &Vec<Vec<Point>> ) -> (f32, f32) {
    let mut w : f32 = 0.0;
    let mut h : f32  = 0.0;

    for model in models {
        let mut minx : f32 = 1e10;
        let mut miny : f32 = 1e10;
        let mut maxx : f32 = -1e10;
        let mut maxy : f32 = -1e10;
        
        for point in model {
            if point.x < minx { minx = point.x; }
            if point.y < miny { miny = point.y; }
            if point.x > maxx { maxx = point.x; }
            if point.y > maxy { maxy = point.y; }
        }
        let tw = (maxx - minx).abs();
        let th = (maxy - miny).abs();
        if tw > w { w = tw; }
        if th > h { h = th; }
    }

    (w, h)
}

/// Returns a Vector of Vectors of Points
/// Filter models, by cutoff size thus far
/// 
/// # Arguments
/// 
/// * `models` - A Vec of Vectors of Point
/// * `cutoff` - a u32 representing  the cutoff number of points
/// * `accepted` - a Vec of usize representing the indices of the models we have accepted.
///
fn filter_models(models : & Vec<Vec<Point>>, cutoff: u32, accepted : Vec<usize>) -> Vec<Vec<Point>> {
    let mut idx = 0;
    let mut accepted_models : Vec<Vec<Point>> = vec!();
    while idx < models.len() {
        let mut remove : bool = false;
        if accepted.len() > 0 {
            if !accepted.contains(&idx) { remove = true; }
        }

        if models[idx].len() < cutoff as usize { remove = true; }
        if !remove {
            let cc = models[idx].clone(); 
            accepted_models.push(cc);
        }
        idx = idx + 1;
    }
    accepted_models
}

/// Returns statistics on the model as a tuple: mean, median, stddev, min and max
/// Get some stats on the models, starting with the mean and
/// median number of points
/// 
/// # Arguments
/// 
/// * `models` - A Vec of Vectors of Point
///
fn find_stats ( models : &Vec<Vec<Point>> ) -> (f32, u32, f32, u32, u32) {
    let mut mean : f32 = 0.0;
    let mut median : u32 = 0;
    let mut min : u32 = 100000000;
    let mut max : u32 = 0;
    let mut sd : f32 = 0.0;
    let mut vv : Vec<u32> = vec![];

    for model in models {
        let ll = model.len();
        vv.push(ll as u32);
        if (ll as u32) < min {
            min = ll as u32;
        } 
        if (ll as u32) > max {
            max = ll as u32;
        }
        mean = mean + model.len() as f32;
    }
    
    vv.sort();
    median = vv[ (vv.len() / 2) as usize] as u32;
    mean = mean / vv.len() as f32;
    let vlen = vv.len();

    for ll in vv {
        sd = (ll as f32 - mean) * (ll as f32 - mean);
    }
    sd = (sd / vlen as f32).sqrt();
    
    (mean, median, sd, min, max)
}

/// Returns a Vec of Point - the model
/// Scale and move all the points so they are in WIDTH, HEIGHT
/// and the Centre of mass moves to the origin.
/// We pass in the global scale as we don't want to scale per image.
/// We are moving the centre of mass to the centre of the image though
/// so we have to put in translation to our final model
/// 
/// # Arguments
/// 
/// * `models` - A Vec of Vectors of Point
/// * `scale` - An f32 representing the scale for the points
///
fn scale_shift_model( model : &Vec<Point>, scale : f32 ) -> Vec<Point> {
    let mut scaled : Vec<Point> = vec![];
    let mut minx : f32 = 1e10;
    let mut miny : f32 = 1e10;
    let mut maxx : f32 = -1e10;
    let mut maxy : f32 = -1e10;

    for point in model {
        if point.x < minx { minx = point.x; }
        if point.y < miny { miny = point.y; }
        if point.x > maxx { maxx = point.x; }
        if point.y > maxy { maxy = point.y; }
    }

    let com = ((maxx + minx) / 2.0, (maxy + miny) / 2.0);
    /*let diag =((maxx - minx) * (maxx - minx) + (maxy - miny) * (maxy - miny)).sqrt();
    // Make scalar a little smaller after selecting the smallest
    let scalar = (WIDTH as f32 / diag).min(HEIGHT as f32 / diag) * SHRINK;*/
    let scalar = scale * (WIDTH as f32) * SHRINK;
        
     for point in model {
        let np = Point {
            x : (point.x - com.0) * scalar,
            y : (point.y - com.1) * scalar
        };
        scaled.push(np);
    } 
    scaled
}

/// Returns None
/// Save a fits image
/// # Arguments
/// 
/// * `img` - A Vec of Vectors of f32 - the pixels
/// * `filename` - A String - the filename to save
///
pub fn save_fits(img : &Vec<Vec<f32>>, filename : &String) {
    let mut data : Vec<f32> = (0..HEIGHT)
        .map(|i| (0..WIDTH).map(
               move |j| (i + j) as f32)).flatten().collect();

    for _y in 0..HEIGHT {
        for _x in 0..WIDTH {
            let idx : usize = (_y * WIDTH +_x ) as usize; 
            data[idx] = img[_x as usize][(HEIGHT - _y - 1) as usize];
            // / intensity * MULTFAC;
        }
    }

    let mut primary_hdu = 
        Hdu::new(&[WIDTH as usize , HEIGHT as usize], data);
    // Insert values in header
    primary_hdu.insert("NORMALISATION", "NONE");
    primary_hdu.insert("WIDTH", WIDTH as i32);
    primary_hdu.insert("HEIGHT", HEIGHT as i32);
    Fits::create(filename, primary_hdu).expect("Failed to create");  
}

/// Returns a Vec of Point - a model
/// Drop points so we are equal to or under a max.
/// # Arguments
/// 
/// * `img` - A Vec of Vectors of Point - a model
/// * `max_points` - A usize representing the maximum number of points
///
/*
pub fn drop_points(img : &Vec<Point>, max_points : usize) -> Vec<Point> {
    let mut fmodel : Vec<Point> = vec!();
    let mut rng = rand::thread_rng();
    let mut choices : Vec<usize> = vec!(); 

    for i in 0..max_points {
        let mut ridx = rng.gen_range(0, img.len()-1);
        while choices.contains(&ridx) {
            ridx = rng.gen_range(0, img.len()-1);
        }

        choices.push(ridx);
        fmodel.push(img[ridx])
    }

    fmodel
}*/

/// Returns a Vec of Point - a model
/// Drop points so we are equal to or under a max.
/// # Arguments
/// 
/// * `models` - A Vec of Vectors of Point - a model
/// * `out_path` - A String representing the path to render to
/// * `nthreads` - A u32 - the number of threads to spin up
/// * `pertubations` - A u32 - how many angles to use in the spin
/// * `sigma` - An f32 - what sigma value to use
/// * `scale` - An f32 - what scale to use
/// * `max_points` - A usize - maximum number of points to 
///
fn render (models : &Vec<Vec<Point>>, out_path : &String,  nthreads : u32, sigma : f32, scale : f32, max_points : usize) {
    // Split into threads here I think
    let pi = std::f32::consts::PI;
    let (tx, rx) = channel();
    let mut progress : i32 = 0;
    let mut pool = Pool::new(nthreads);

    let num_runs = models.len() as u32;
    let truns = (num_runs / nthreads) as u32;
    let spare = (num_runs % nthreads) as u32;
    let mut pb = ProgressBar::new(num_runs as u64);
    pb.format("╢▌▌░╟");

    pool.scoped(|scoped| {
        for _t in 0..nthreads {
            let tx = tx.clone();
            let start : usize = (_t * truns) as usize;
            let mut end = ((_t + 1)  * truns) as usize;
            if _t == nthreads - 1 { end = end + (spare as usize); }
            let cslice = &models[start..end];
           
            scoped.execute( move || { 
                let mut rng = thread_rng();
                let side = Uniform::new(-pi, pi);

                for _i in 0..cslice.len() {
                    // Slightly inefficient if we are dropping points
                    let mut scaled = scale_shift_model(&cslice[_i], scale);
                    //if max_points != 0 {
                    //    let fslice = drop_points(&cslice[_i], max_points);
                    //    scaled = scale_shift_model(&fslice, scale);
                    //}
                    
             
                    let mut timg : Vec<Vec<f32>> = vec![];

                    // Could be faster I bet
                    for _x in 0..WIDTH {
                        let mut tt : Vec<f32> = vec![];
                        for _y in 0..HEIGHT { tt.push(0.0); }
                        timg.push(tt);
                    }
                    // A random rotation around the plane
                    let rr = rng.sample(side);
                    let rm = (rr.cos(), -rr.sin(), rr.sin(), rr.cos());

                    for ex in 0..WIDTH {
                        for ey in 0..HEIGHT {
                            for point in &scaled {
                                let xs = point.x * rm.0 + point.y * rm.1;
                                let ys = point.x * rm.2 + point.y * rm.3;
                                let xf = xs + (WIDTH as f32/ 2.0);
                                let yf = ys + (HEIGHT as f32 / 2.0);
                                if xf >= 0.0 && xf < WIDTH as f32 && yf >= 0.0 && yf < HEIGHT as f32 {   
                                    let pval = (1.0 / (2.0 * pi * sigma.powf(2.0))) *
                                        (-((ex as f32 - xf).powf(2.0) + (ey as f32 - yf).powf(2.0)) / (2.0*sigma.powf(2.0))).exp();        
                                    timg[ex as usize][ey as usize] += pval;
                                }
                                // We may get ones that exceed but it's very likely they are outliers
                                /*else {
                                    // TODO - ideally we send an error that propagates
                                    // and kills all other threads and quits cleanly
                                    println!("Point still exceeding range in image");
                                }*/
                            }
                        }
                    }
                    
                    let fidx = format!("/image_{:06}.fits",
                        ((start + _i) as usize));
                    let mut fitspath = out_path.clone();
                    fitspath.push_str(&fidx);
                    save_fits(&timg, &fitspath);
                    tx.send(_i).unwrap();
                }
            });
        }

        // Update our progress bar
        while progress < num_runs as i32 {
            match rx.try_recv() {
                Ok(_a) => {
                    pb.inc();
                    progress = progress + 1;
                }, Err(_e) => {}
            }
        }
    });
}

/// Returns a Result of Vec of Vec of Point.
/// Parse the CSV file
/// # Arguments
/// 
/// * `path` - A String - the path to the CSV file
///
fn parse_csv(path : &String) -> Result<Vec<Vec<Point>>, Box<Error>> {
    let mut models : Vec<Vec<Point>>  = vec!();
    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut model : Vec<Point> = vec![];

    for result in rdr.records() {
        let record = result?;
        //println!("{:?}", record);
        let x: f32 = record[0].parse()?;
        let y: f32 = record[1].parse()?;

        let p = Point {
            x : x,
            y : y
        };
        model.push(p);
    }
    models.push(model);
    Ok(models)
}

fn main() {
     let args: Vec<_> = env::args().collect();
    
    if args.len() < 5 {
        println!("Usage: render <path to csv file> <path to output> <threads> <sigma>"); 
        process::exit(1);
    }
    
    let nthreads = args[3].parse::<u32>().unwrap();
    let sigma = args[4].parse::<f32>().unwrap();
    let mut accepted : Vec<usize> = vec!();
    let mut max_points : usize = 0;

   
    match parse_csv(&args[1]) {
        Ok(mut models) => {
            let (w, h) = find_extents(&models);
            let (mean, median, sd, min, max) = find_stats(&models);
            let cutoff = median - ((2.0 * sd) as u32);
            let accepted_models = filter_models(&models, cutoff, accepted);
            // Find extents a second time
            let (w, h) = find_extents(&accepted_models);
            let (mean, median, sd, min, max) = find_stats(&accepted_models);
            let cutoff = median - ((2.0 * sd) as u32);
            println!("Model sizes (min, max, mean, median, sd) : {}, {}, {}, {}, {}", 
                min, max, mean, median, sd);
            let mut scale = 2.0 / w;
            if h > w { scale = 2.0 / h; }
            println!("Max Width / Height: {}, {}", w, h);
            println!("Scale / Scalar: {}, {}", scale, scale * (WIDTH as f32) * SHRINK); 
            render(&accepted_models, &args[2], nthreads, sigma, scale, max_points);
        }, 
        Err(e) => {
            println!("Error parsing MATLAB File: {}", e);
        }
    }
}
