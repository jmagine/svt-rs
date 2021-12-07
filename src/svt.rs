use anyhow::{anyhow, Result, Context};

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
    let exp = ui_ctrl.exponent_text.text().parse::<f32>().context("[apply] invalid exponent")?;
    let t_off = ui_ctrl.offset_text.text().parse::<i32>().context("[apply] invalid offset")?;
    let t_buf = ui_ctrl.buffer_text.text().parse::<i32>().context("[apply] invalid buffer")?;

    let start_obj = create_map_object(start_line.to_string(), true).context("[apply] timing point format error")?;
    let end_obj = create_map_object(end_line.to_string(), true).context("[apply] timing point format error")?;

    if (ui_ctrl.sv_check.check_state(), ui_ctrl.vol_check.check_state()) == (nwg::CheckBoxState::Unchecked, nwg::CheckBoxState::Unchecked) {
      return Err(anyhow!("[apply] nothing to apply (sv, vol)"));
    }

    //determine bpm at starting/ending point
    let mut s_bpm = 160.0;
    let mut e_bpm = 160.0;
    for obj in self.all_objs.iter() {
      if obj.class == 0 {
        if obj.time <= start_obj.time {
          s_bpm = 60000.0 / obj.beatlength;
        }
        if obj.time <= end_obj.time {
          e_bpm = 60000.0 /  obj.beatlength;
        }
      }
    }

    //convert beatlength values to sv values
    let s_sv_raw = -100.0 * s_bpm / start_obj.beatlength;
    let e_sv_raw = if ui_ctrl.eq_bpm_check.check_state() == nwg::CheckBoxState::Checked {
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
    let v_diff = end_obj.volume - start_obj.volume;
    let sv_per_ms = sv_diff / t_diff as f32;
    let v_per_ms = v_diff as f32 / t_diff as f32;

    let mut bpm = 160.0;
    let mut beatlength = -100.0;
    let mut meter = 4;
    let mut sample_set = 0;
    let mut sample_index = 0;
    let mut volume = 100;
    let mut effects = 0;

    for obj in self.all_objs.iter() {
      //set fields before performing calculations
      if obj.class == 0 {
        //uninherited point
        bpm = 60000.0 / obj.beatlength;
        meter = obj.meter;

        beatlength = obj.beatlength;
        sample_set = obj.sampleset;
        sample_index = obj.sampleindex;
        volume = obj.volume;
        effects = obj.effects;
        continue;
      }
      
      if obj.class == 1 {
        //either uninherited or inherited point
        beatlength = obj.beatlength;
        sample_set = obj.sampleset;
        sample_index = obj.sampleindex;
        volume = obj.volume;
        effects = obj.effects;
      }

      //perform general calculations here for inher, barlines, hitobjects
      let obj_time = obj.time;
      if obj_time >= start_obj.time - t_buf && obj_time <= end_obj.time + t_buf {
        let new_t = obj_time + t_off;
        let new_sv = if ui_ctrl.exponential_check.check_state() == nwg::CheckBoxState::Checked {
          //exponential
          s_sv_raw + sv_diff * f32::powf((obj_time - start_obj.time) as f32 / t_diff as f32, exp)
        } else {
          //linear
          s_sv_raw + (obj_time - start_obj.time) as f32 * sv_per_ms
        };

        let new_b = -100.0 / (new_sv / bpm);
        let new_v = ((start_obj.volume as f32 + (obj_time - start_obj.time) as f32 * v_per_ms)).round() as u32;
        let new_point = match (ui_ctrl.sv_check.check_state(), ui_ctrl.vol_check.check_state()) {
          //sv and vol
          (nwg::CheckBoxState::Checked, nwg::CheckBoxState::Checked) => {
            format!("{},{},{},{},{},{},{},{}", new_t, new_b, meter, sample_set, sample_index, new_v, 0, effects)
          },
          //sv and no vol
          (nwg::CheckBoxState::Checked, nwg::CheckBoxState::Unchecked) => {
            format!("{},{},{},{},{},{},{},{}", new_t, new_b, meter, sample_set, sample_index, volume, 0, effects)
          },
          //no sv and vol
          (nwg::CheckBoxState::Unchecked, nwg::CheckBoxState::Checked) => {
            format!("{},{},{},{},{},{},{},{}", new_t, beatlength, meter, sample_set, sample_index, new_v, 0, effects)
          },
          //no sv, no vol - should not reach this point
          _ => {format!("")},
        };

        match obj.class {
          0 => {println!("[apply] shouldn't get here, class 1");}, //uninherited line
          1 => {
            //inherited line
            if ui_ctrl.inh_check.check_state() == nwg::CheckBoxState::Checked {
              println!("[new] inh {}", new_point);
              self.new_objs.push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          2 => {
            //barline
            if ui_ctrl.barline_check.check_state() == nwg::CheckBoxState::Checked {
              println!("[new] bar {}", new_point);
              self.new_objs.push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          3 => {
            //hitobject
            if ui_ctrl.hit_check.check_state() == nwg::CheckBoxState::Checked {
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

  //clear all old map objects, load in a new file and repopulate with latest saved state
  pub fn load_file(&mut self, filename: &String) {
    let mut bool_timing = false;
    let mut bool_hit = false;

    //TODO figure out better way to init this
    let mut bar_time: f32 = 100000.0;
    let mut bar_inc: f32 = 100000.0;

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
                    self.all_objs.push(MapObject{time: bar_time as i32, class: 2, data: String::from(""), ..Default::default()});
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