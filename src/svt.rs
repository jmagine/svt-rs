use nwg::CheckBoxState::{Checked, Unchecked};

use anyhow::{anyhow, Result, Context};

use std::cmp;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::ui;

#[derive(Clone, Debug, Default)]
pub struct MapObject {
  pub class: i32,
  pub time: i32,
  pub beatlength: f32,
  pub meter: i32,
  pub sampleset: i32,
  pub sampleindex: i32,
  pub volume: i32,
  pub uninherited: i32,
  pub effects: i32,
  pub data: String,
}

#[derive(Default)]
pub struct SVT {
  pub all_objs: Vec<MapObject>,
  pub new_objs: Vec<MapObject>,
}

impl SVT {
  //TODO figure out something better than passing ui struct in lol
  //apply timing between two points
  pub fn apply_timing(&mut self, start_line: &str, end_line: &str, ui_ctrl: &ui::UI) -> Result<()> {
    
    //only validate these text fields when the corresponding modes are enabled
    let pol_exp = if ui_ctrl.pol_sv_check.check_state() == Checked {
      ui_ctrl.pol_exp_text.text().parse::<f32>().context("[apply] invalid exponent")?
    } else {
      1.0
    };
    let flat_sv = if ui_ctrl.flat_sv_check.check_state() == Checked {
      ui_ctrl.flat_sv_text.text().parse::<f32>().context("[apply] invalid flat sv")?
    } else {
      0.0
    };

    let t_off = ui_ctrl.offset_text.text().parse::<i32>().context("[apply] invalid offset")?;
    let t_buf = ui_ctrl.buffer_text.text().parse::<i32>().context("[apply] invalid buffer")?;

    let start_obj = create_map_object(start_line.to_string(), true).context("[apply] timing point format error")?;
    let end_obj = create_map_object(end_line.to_string(), true).context("[apply] timing point format error")?;

    //TODO this can probably happen in UI, not here
    let sv_change_bool: bool = ui_ctrl.lin_sv_check.check_state() == Checked || 
        ui_ctrl.pol_sv_check.check_state() == Checked || 
        ui_ctrl.exp_sv_check.check_state() == Checked || 
        ui_ctrl.flat_sv_check.check_state() == Checked;
    if (sv_change_bool, ui_ctrl.vol_check.check_state()) == (false, Unchecked) {
      return Err(anyhow!("[apply] nothing to apply (sv, vol)"));
    }

    //TODO although all_objs is sorted at this point, sort can be moved here for clarity/guarantee

    //initial pass of all map objects to determine bpm at starting/ending point
    let mut s_bpm = 0.0;
    let mut e_bpm = 0.0;
    for obj in self.all_objs.iter() {
      if obj.class == 0 {
        if obj.time <= start_obj.time {
          s_bpm = 60000.0 / obj.beatlength;
        }
        if obj.time <= end_obj.time {
          e_bpm = 60000.0 / obj.beatlength;
        }
      }
    }

    if s_bpm == 0.0 {
      return Err(anyhow!("[apply] no uninherited lines detected"));
    }

    //convert beatlength values to sv values
    let s_sv_raw = -100.0 * s_bpm / start_obj.beatlength;
    let e_sv_raw = if ui_ctrl.ign_bpm_check.check_state() == Checked {
      -100.0 * s_bpm / end_obj.beatlength
    } else {
      -100.0 * e_bpm / end_obj.beatlength
    };

    //debug print
    println!("[apply] t:{}->{} raw sv:{}->{} vol:{}->{}", start_obj.time, end_obj.time, s_sv_raw, e_sv_raw, start_obj.volume, end_obj.volume);

    //validation on input values
    if start_obj.time > end_obj.time {
      return Err(anyhow!("[apply] invalid times (end <= start)"));
    }
    if s_sv_raw <= 0.0 || e_sv_raw <= 0.0 {
      return Err(anyhow!("[apply] invalid sv value(s) (sv <= 0)"));
    }
    if start_obj.volume < 0 || start_obj.volume > 100 || end_obj.volume < 0 || end_obj.volume > 100 {
      return Err(anyhow!("[apply] invalid volumes (vol < 0 or vol > 100)"));
    }
    
    //compute change per time tick
    let t_diff = end_obj.time - start_obj.time;
    let sv_diff = e_sv_raw - s_sv_raw;
    let sv_ratio = e_sv_raw / s_sv_raw;
    let vol_diff = end_obj.volume - start_obj.volume;
    let sv_per_ms = sv_diff / t_diff as f32;
    let vol_per_ms = vol_diff as f32 / t_diff as f32;

    //TODO update these with the real default values
    //init with something here to prevent catastrophic failure before first uninherited line
    let mut last_uni_time = 0;
    let mut bpm = 160.0;
    let mut beatlength = -100.0;
    let mut meter = 4;
    let mut sample_set = 0;
    let mut sample_index = 0;
    let mut volume = 100;
    let mut effects = 0;

    let mut kiai_change_time = 0;

    for obj in self.all_objs.iter() {
      //only consider timing points for flat sv
      if ui_ctrl.flat_sv_check.check_state() == Checked && obj.class > 1 {
        continue;
      }

      //set fields before performing calculations
      if obj.class == 0 {
        //uninherited point

        //check whether kiai change occurs
        if obj.effects & 1 != effects & 1 {
          kiai_change_time = obj.time;
        }

        last_uni_time = obj.time;

        bpm = 60000.0 / obj.beatlength;
        meter = obj.meter;

        beatlength = obj.beatlength;
        sample_set = obj.sampleset;
        sample_index = obj.sampleindex;
        volume = obj.volume;
        effects = obj.effects;
        continue;
      } else if obj.class == 1 {
        //inherited point

        //check whether kiai change occurs
        if obj.effects & 1 != effects & 1 {
          kiai_change_time = obj.time;
        }

        beatlength = obj.beatlength;
        sample_set = obj.sampleset;
        sample_index = obj.sampleindex;
        volume = obj.volume;
        effects = obj.effects;
      }

      //perform general calculations here for inher, barlines, hitobjects
      let obj_time = obj.time;
      if obj_time >= start_obj.time - t_buf && obj_time <= end_obj.time + t_buf {
        //ensure time is set both after any uninherited points or kiai time changes within offset window
        let new_t = cmp::max(cmp::max(obj_time + t_off, last_uni_time), kiai_change_time);
        let new_sv = if ui_ctrl.lin_sv_check.check_state() == Checked {
          //linear
          s_sv_raw + (obj_time - start_obj.time) as f32 * sv_per_ms
        } else if ui_ctrl.exp_sv_check.check_state() == Checked {
          //exponential
          s_sv_raw * f32::exp((obj_time - start_obj.time) as f32 * f32::ln(sv_ratio) / t_diff as f32)
        } else if ui_ctrl.pol_sv_check.check_state() == Checked {
          //polynomial
          s_sv_raw + sv_diff * f32::powf(cmp::max(0, obj_time - start_obj.time) as f32 / t_diff as f32, pol_exp)
        } else if ui_ctrl.flat_sv_check.check_state() == Checked {
          //flat
          (-100.0 / obj.beatlength + flat_sv) * s_bpm
        } else {
          -100.0
        };

        let new_b = -100.0 / (new_sv / bpm);
        let new_vol = ((start_obj.volume as f32 + (obj_time - start_obj.time) as f32 * vol_per_ms)).round() as u32;
        let new_point = match (sv_change_bool, ui_ctrl.vol_check.check_state()) {
          //sv and vol
          (true, Checked) => {
            format!("{},{},{},{},{},{},{},{}", new_t, new_b, meter, sample_set, sample_index, new_vol, 0, effects)
          },
          //sv and no vol
          (true, Unchecked) => {
            format!("{},{},{},{},{},{},{},{}", new_t, new_b, meter, sample_set, sample_index, volume, 0, effects)
          },
          //no sv and vol
          (false, Checked) => {
            format!("{},{},{},{},{},{},{},{}", new_t, beatlength, meter, sample_set, sample_index, new_vol, 0, effects)
          },
          //no sv, no vol - should not reach this point
          _ => {
            format!("")
          },
        };

        match obj.class {
          0 => {println!("[apply] shouldn't get here, class 1");}, //uninherited line
          1 => {
            //inherited line
            if ui_ctrl.inh_check.check_state() == Checked || ui_ctrl.flat_sv_check.check_state() == Checked {
              println!("[new] inh {}", new_point);
              self.new_objs.push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          2 => {
            //barline
            if ui_ctrl.barline_check.check_state() == Checked {
              println!("[new] bar {}", new_point);
              self.new_objs.push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          3 => {
            //hitobject
            if ui_ctrl.hit_check.check_state() == Checked {
              println!("[new] hit {}", new_point);
              self.new_objs.push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          _ => {
            println!("[apply] unknown class {}", obj.class);
          }
        }
      }
    }
    Ok(())
  }

  //TODO could return a value to indicate whether load was successful
  //can reduce issues related to diff name changing, file DNE, etc.
  //clear all old map objects, load in a new file and repopulate with latest saved state
  pub fn load_osu(&mut self, filename: &String) {
    let mut bool_timing = false;
    let mut bool_hit = false;

    //TODO figure out better way to init this
    let mut bar_time: f32 = 1000000.0;
    let mut bar_inc: f32 = 1000000.0;

    self.all_objs.clear();
    self.new_objs.clear();

    // read file line by line
    if let Ok(lines) = read_lines(&filename) {
      for line in lines {
        if let Ok(s) = line {
          // we only care about the TimingPoints and HitObjects headers
          match s.as_str() {
            "[General]" | "[Editor]" | "[Metadata]" | "[Difficulty]" | "[Events]" | "[Colours]" => {
              bool_timing = false;
              bool_hit = false;
            },
            "[TimingPoints]" => {
              bool_timing = true;
              bool_hit = false;
              println!("[load] found [TimingPoints], reading");
            },
            "[HitObjects]" => {
              bool_timing = false;
              bool_hit = true;
              println!("[load] found [HitObjects], reading");
            },
            _ => {
              if bool_timing {
                if let Ok(map_obj) = create_map_object(s, true) {
                  //add barlines since last timing point
                  while bar_time + bar_inc < map_obj.time as f32 {
                    bar_time += bar_inc;
                    self.all_objs.push(MapObject{time: bar_time.round() as i32, class: 2, data: String::from(""), ..Default::default()});
                  }

                  //use uninherited point properties to calculate barline times
                  if map_obj.uninherited == 1 {
                    //set barline counter
                    bar_time = map_obj.time as f32;
                    bar_inc = map_obj.beatlength * map_obj.meter as f32;

                    //add current barline if not skipping barline (skip if effects is set to 8)
                    if map_obj.effects & 8 != 8 {
                      self.all_objs.push(MapObject{time: bar_time as i32, class: 2, data: String::from(""), ..Default::default()});
                    }
                  }

                  //add timing point
                  self.all_objs.push(map_obj);
                }
              } else if bool_hit {
                if let Ok(map_obj) = create_map_object(s, false) {
                  self.all_objs.push(map_obj);
                }
              } else {
                continue;
              }
            },
          }
        }
      }
    }

    self.all_objs.sort_by_key(|k| (k.time, k.class));
  }

  //write the current output points to the destination file, using the input file as a template for everything except timing points
  pub fn write_output_points(&mut self, in_filename: String, out_filename: String, preview: bool) -> Result<()> {
    //don't write anything if no new objects
    if self.new_objs.len() == 0 {
      return Err(anyhow!("[write] no new objects to apply"));
    }
    
    //sort new objects in chronological order
    //TODO inherited vs uninherited should not apply here, these are the new points
    self.new_objs.sort_by_key(|k| (k.time, k.class));

    //build up a vector with all old and new points sorted in chronological, then uninherited > inherited order
    let mut out_objs: Vec<MapObject> = Vec::new();
    out_objs.extend(self.new_objs.iter().cloned());
    for obj in self.all_objs.iter() {
      //uninherited/inherited lines
      if obj.class == 0 || obj.class == 1 {
        out_objs.push(obj.clone());
      }
    }

    //uninherited ^ 1 indicates priority, while (time, uninherited) should be unique
    out_objs.sort_by_key(|k| (k.time, k.uninherited ^ 1));
    out_objs.dedup_by_key(|k| (k.time, k.uninherited));

    //make backup before writing file, don't write without backing up
    if let Err(e) = fs::copy(&in_filename, "backup.osu") {
      return Err(anyhow!("[backup] error backing up file {}", e));
    }

    // read file line by line
    let mut out_string = String::new();
    let mut bool_timing = false;
    if let Ok(lines) = read_lines(&in_filename) {
      for line in lines {
        if let Ok(s) = line {
          // we want everything except timingpoints lines
          match s.as_str() {
            "[General]" | "[Editor]" | "[Metadata]" | "[Difficulty]" | "[Events]" | "[Colours]" | "[HitObjects]" => {
              bool_timing = false;
              out_string += &s;
              out_string += "\n";
            },
            "[TimingPoints]" => {
              bool_timing = true;
              out_string += &s;
              out_string += "\n";
              for out_obj in out_objs.iter() {
                out_string += &out_obj.data;
                out_string += "\n";
              }
              out_string += "\n";
            },
            _ => {
              let s_tokens: Vec<&str> = s.split(":").collect();
              if preview && s_tokens.len() == 2 && s_tokens[0] == "Version" {
                out_string += "Version:preview\n";
              } else if !bool_timing {
                out_string += &s;
                out_string += "\n";
              }
            },
          }
        }
      }
    } else {
      return Err(anyhow!("[write] input file/filename invalid"));
    }

    let mut out_file = File::create(out_filename).unwrap();
    let _ = write!(&mut out_file, "{}", out_string);
    Ok(())
  }

  pub fn print_debug(&self) {
    println!("\n[svt] DEBUG all_objs:");

    let mut uni_count = 0;
    let mut inh_count = 0;
    let mut bar_count = 0;
    let mut hit_count = 0;

    for map_obj in self.all_objs.iter() {
      match map_obj.class {
        0 => {
          println!("[svt] uni {}", map_obj.data.trim());
          uni_count += 1;
        },
        1 => {
          println!("[svt] inh {}", map_obj.data.trim());
          inh_count += 1;
        },
        2 => {
          println!("[svt] bar {}", map_obj.time);
          bar_count += 1;
        },
        3 => {
          //println!("[svt] hit {}", map_obj.time);
          hit_count += 1;
        },
        _ => {
          println!("[svt] ???");
        },
      }
    }
    println!("[svt] counts:\nuni: {}\ninh: {}\nbar: {}\nhit: {}\n", uni_count, inh_count, bar_count, hit_count);
  }
}

//creates a MapObject for use with the svt module
fn create_map_object(p: String, timingpoint: bool) -> Result<MapObject> {
  let p_tokens: Vec<&str> = p.split(",").collect();

  let map_obj = if timingpoint {
    //timing point
    if p_tokens.len() != 8 {
      return Err(anyhow!("[create] timing: incorrect format {}", p));
    }
  
    let time = p_tokens[0].parse::<i32>()?;
    let beatlength = p_tokens[1].parse::<f32>()?;
    let meter = p_tokens[2].parse::<i32>()?;
    let sampleset = p_tokens[3].parse::<i32>()?;
    let sampleindex = p_tokens[4].parse::<i32>()?;
    let volume = p_tokens[5].parse::<i32>()?;
    let uninherited = p_tokens[6].parse::<i32>()?;
    let effects = p_tokens[7].parse::<i32>()?;

    MapObject{class: uninherited ^ 1, time: time, beatlength: beatlength, meter: meter, sampleset: sampleset, sampleindex: sampleindex, volume: volume, uninherited: uninherited, effects: effects, data: p}
  } else {
    //hit point
    if p_tokens.len() < 2 {
      return Err(anyhow!("[create] hit: incorrect format {}", p));
    }

    let time = p_tokens[2].parse::<i32>()?;

    MapObject{class: 3, time: time, data: String::from(""), ..Default::default()}
  };

  return Ok(map_obj);
}

//convenience function for reading file line by line
fn read_lines<P>(full_path: P) -> io::Result<io::Lines<io::BufReader<File>>> where P: AsRef<Path>, {
  let file = File::open(full_path)?;
  Ok(io::BufReader::new(file).lines())
}