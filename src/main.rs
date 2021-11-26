//#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use anyhow::{anyhow, Result, Context};
use nwd::NwgUi;
use nwg::NativeUi;
use std::{cell::RefCell};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::cmp;

#[derive(Clone, Default)]
pub struct MapObject {
  class: i32,
  time: i32,
  beatlength: f32,
  meter: i32,
  sampleset: i32,
  sampleindex: i32,
  volume: i32,
  uninherited: i32,
  effects: i32,
  data: String,
}

/*
impl Default for MapObject {
  fn default() -> MapObject {
    MapObject {
      class: 0,
      time: 0,
      beatlength: 0,
    }
  }
}
*/

#[derive(Default, NwgUi)]
pub struct SVT {
  //TODO make this pixel-perfect pretty
  #[nwg_control(size: (300, 300), position: (cmp::max(0, nwg::Monitor::width() / 2 - 150), cmp::max(0, nwg::Monitor::height() / 2 - 150)), title: "SVT", accept_files: true, flags: "WINDOW|VISIBLE|MINIMIZE_BOX")]
  #[nwg_events( OnWindowClose: [SVT::close_window], OnFileDrop: [SVT::drop_file(SELF, EVT_DATA)] )]
  window: nwg::Window,

  //timing point input
  #[nwg_control(text: "", size: (290, 50), position: (5,5), flags: "VISIBLE|AUTOVSCROLL|TAB_STOP")]
  inherited_text: nwg::TextBox,

  //apply
  #[nwg_control(size: (100, 85), position: (5, 60))]
  apply_frame: nwg::Frame,

  #[nwg_control(text: "Apply to:", size: (95, 20), position: (2, 0), parent: apply_frame)]
  apply_label: nwg::Label,

  //toggles note changes
  #[nwg_control(text: "Hits", size: (95, 20), position: (2, 20), check_state: CheckBoxState::Checked, parent: apply_frame)]
  hit_check: nwg::CheckBox,

  //toggles barline changes
  #[nwg_control(text: "Barlines", size: (95, 20), position: (2, 40), check_state: CheckBoxState::Checked, parent: apply_frame)]
  barline_check: nwg::CheckBox,

  //toggles inh line changes
  #[nwg_control(text: "Inherited lines", size: (95, 20), position: (2, 60), check_state: CheckBoxState::Unchecked, parent: apply_frame)]
  inh_check: nwg::CheckBox,

  //options
  #[nwg_control(size: (185, 85), position: (110, 60))]
  options_frame: nwg::Frame,

  #[nwg_control(text: "Options:", size: (95, 20), position: (2, 0), parent: options_frame)]
  options_label: nwg::Label,

  //offset time
  #[nwg_control(text: "0", size: (19, 19), position: (2, 20), parent: options_frame)]
  offset_text: nwg::TextInput,

  #[nwg_control(text: "Offset", size: (45, 20), position: (25, 22), parent: options_frame)]
  offset_label: nwg::Label,

  //buffer time
  #[nwg_control(text: "3", size: (19, 19), position: (2, 40), parent: options_frame)]
  buffer_text: nwg::TextInput,

  #[nwg_control(text: "Buffer", size: (45, 20), position: (25, 42), parent: options_frame)]
  buffer_label: nwg::Label,

  //exponential factor
  #[nwg_control(text: "0.5", size: (19, 19), position: (2, 60), parent: options_frame)]
  exponent_text: nwg::TextInput,

  #[nwg_control(text: "Exp.", size: (45, 20), position: (25, 62), parent: options_frame)]
  exponent_label: nwg::Label,

  //toggles end line/start line BPM
  #[nwg_control(text: "Ignore end BPM", size: (105, 20), position: (75, 20), check_state: CheckBoxState::Unchecked, parent: options_frame)]
  eq_bpm_check: nwg::CheckBox,

  //toggles exponential mode
  #[nwg_control(text: "Exponential SV", size: (105, 20), position: (75, 40), check_state: CheckBoxState::Unchecked, parent: options_frame)]
  exponential_check: nwg::CheckBox,

  //select map button
  #[nwg_control(text: "Select Mapfile", size: (102, 25), position: (4, 185))]
  #[nwg_events( OnButtonClick: [SVT::open_file_browser] )]
  open_button: nwg::Button,

  //input map filename
  #[nwg_control(text: "", size: (185, 23), position: (110, 186), flags: "VISIBLE|DISABLED")]
  in_filename: nwg::TextInput,

  //toggles preview
  #[nwg_control(text: "Preview Output", size: (102, 25), position: (5, 215), check_state: CheckBoxState::Checked)]
  #[nwg_events(OnButtonClick: [SVT::fill_out_filename])]
  preview_check: nwg::CheckBox,
  
  //output map filename
  #[nwg_control(text: "", size: (185, 23), position: (110, 216))]
  out_filename: nwg::TextInput,
  
  //place apply button near bottom
  #[nwg_control(text: "Apply", size: (292, 25), position: (4, 245))]
  #[nwg_events( OnButtonClick: [SVT::apply_changes] )]
  hello_button: nwg::Button,

  //place status bar at the very bottom
  #[nwg_control(text: "[map] no map selected (Select Mapfile or drag one in)")]
  status: nwg::StatusBar,

  //open file dialog
  #[nwg_resource(title: "Open File", action: nwg::FileDialogAction::Open, filters: "osu(*.osu)")]
  file_dialog: nwg::FileDialog,

  all_objs: RefCell<Vec<MapObject>>,
  new_objs: RefCell<Vec<MapObject>>,
}

impl SVT {
  fn apply_changes(&self) {
    //refresh file before doing anything
    self.load_file();

    //clear any previously applied points
    self.new_objs.borrow_mut().clear();

    let cmd = self.inherited_text.text();
    let mut lines = cmd.split_whitespace();
    let mut start_line;
    let mut end_line;

    //process 2 valid lines at a time until no lines left
    loop {
      //TODO could also attempt to do more data validation at this step
      //could create the mapobjects here and pass those into apply_timing instead
      start_line = lines.next();
      end_line = lines.next();
      if let (Some(start_l), Some(end_l)) = (start_line, end_line) {
        if let Err(err) = self.apply_timing(start_l, end_l) {
          println!("[apply] error applying timing {}->{}", start_l, end_l);
          self.status.set_text(0, &err.to_string());
          return;
        }
      } else {
        println!("[apply] no more lines");
        break;
      }
    }

    //don't write anything if no new objects
    if self.new_objs.borrow().len() == 0 {
      println!("[apply] no new objects, early termination");
      self.status.set_text(0, "[apply] no new objects to apply");
      return;
    }

    //merge new points into old ones - delete old point if new one is identical
    if let Err(err) = self.write_output_points() {
      println!("[apply] error writing output");
      self.status.set_text(0, &err.to_string());
      return;
    }

    //update status bar with change count on success
    self.status.set_text(0, &format!("[apply] {} changes applied", self.new_objs.borrow().len()));
  }

  fn apply_timing(&self, start_line: &str, end_line: &str) -> Result<()> {
    let exp = self.exponent_text.text().parse::<f32>().context("[apply] invalid exponent")?;
    let t_off = self.offset_text.text().parse::<i32>().context("[apply] invalid offset")?;
    let t_buf = self.buffer_text.text().parse::<i32>().context("[apply] invalid buffer")?;

    let start_obj = create_map_object(start_line.to_string(), true).context("[apply] timing point format error")?;
    let end_obj = create_map_object(end_line.to_string(), true).context("[apply] timing point format error")?;

    //determine bpm at starting/ending point
    let mut s_bpm = 160.0;
    let mut e_bpm = 160.0;
    for obj in self.all_objs.borrow().iter() {
      if obj.class == 1 {
        if obj.time < start_obj.time {
          s_bpm = 60000.0 / obj.beatlength;
        }
        if obj.time < end_obj.time {
          e_bpm = 60000.0 /  obj.beatlength;
        }
      }
    }

    //convert beatlength values to sv values
    let s_sv_raw = -100.0 * s_bpm / start_obj.beatlength;
    let e_sv_raw = if self.eq_bpm_check.check_state() == nwg::CheckBoxState::Checked {
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
    let mut meter = 4;
    let mut sample_set = 0;
    let mut sample_index = 0;
    let mut effects = 0;

    for obj in self.all_objs.borrow().iter() {
      //set fields before performing calculations
      if obj.class == 1 {
        //uninherited point
        bpm = 60000.0 / obj.beatlength;
        meter = obj.meter;
        sample_set = obj.sampleset;
        sample_index = obj.sampleindex;
        effects = obj.effects;
        continue;
      } else if obj.class == 0 {
        //inherited point
        sample_set = obj.sampleset;
        sample_index = obj.sampleindex;
        effects = obj.effects;
      }

      //perform general calculations here for inher, barlines, hitobjects
      let obj_time = obj.time;
      if obj_time >= start_obj.time - t_buf && obj_time <= end_obj.time + t_buf {
        let new_t = obj_time + t_off;
        let new_sv = if self.exponential_check.check_state() == nwg::CheckBoxState::Checked {
          //exponential
          s_sv_raw + sv_diff * f32::powf((obj_time - start_obj.time) as f32 / t_diff as f32, exp)
        } else {
          //linear
          s_sv_raw + (obj_time - start_obj.time) as f32 * sv_per_ms
        };

        let new_b = -100.0 / (new_sv / bpm);
        let new_v = (start_obj.volume as f32 + (obj_time - start_obj.time) as f32 * v_per_ms) as u32;
        let new_point = format!("{},{},{},{},{},{},{},{}", new_t, new_b, meter, sample_set, sample_index, new_v, 0, effects);

        match obj.class {
          0 => {
            //inherited line
            if self.inh_check.check_state() == nwg::CheckBoxState::Checked {
              println!("[new] inh {}", new_point);
              self.new_objs.borrow_mut().push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          1 => {println!("[apply] shouldn't get here, class 1");}, //uninherited line
          2 => {
            //barline
            if self.barline_check.check_state() == nwg::CheckBoxState::Checked {
              println!("[new] bar {}", new_point);
              self.new_objs.borrow_mut().push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
            }
          },
          3 => {
            //hitobject
            if self.hit_check.check_state() == nwg::CheckBoxState::Checked {
              println!("[new] hit {}", new_point);
              self.new_objs.borrow_mut().push(MapObject{time: new_t, class: 4, data: new_point, ..Default::default()});
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
  
  fn close_window(&self) {
    nwg::stop_thread_dispatch();
  }

  fn drop_file(&self, data: &nwg::EventData) {
    self.in_filename.set_text(&data.on_file_drop().files().pop().unwrap());
    self.fill_out_filename();
    self.load_file();
  }

  fn fill_out_filename(&self) {
    if self.preview_check.check_state() == nwg::CheckBoxState::Checked {
      let in_filename = &self.in_filename.text();

      //prevent paths without parents or filenames from crashing
      let folder = Path::new(in_filename).parent();
      let name_osu = Path::new(in_filename).file_name();

      //TODO check path is valid maybe?
      //TODO clean this match statement up
      match (folder, name_osu) {
        (Some(folder), Some(name_osu)) => {
          self.out_filename.set_text(&format!("{}/{}[{}].osu", folder.to_str().unwrap(), name_osu.to_str().unwrap().split("[").nth(0).unwrap(), "preview"));
        },
        _ => {
          println!("[pre] path invalid");
        },
      }
    } else {
      self.out_filename.set_text(&self.in_filename.text());
    }
  }

  //load and parse .osu file line by line
  fn load_file(&self) {
    let filename = self.in_filename.text();

    //TODO this check is probably not sufficient. need additional validation
    if filename.len() == 0 {
      println!("[load] empty filename");
      return;
    }

    //determine filename and extension
    let ext = Path::new(&filename).extension();

    let folder = String::from(Path::new(&filename).parent().unwrap().to_str().unwrap());
    println!("[load] folder: {}", folder);

    //skip any file that is not .osu
    match ext {
      Some(str) => if str.to_str() != Some("osu") {
        println!("[load] incorrect file type: .{}", str.to_str().unwrap());
        return},
      None => {
        return
      },
    }

    println!("[load] loading {}", Path::new(&filename).file_name().unwrap().to_str().unwrap());

    let mut bool_timing = false;
    let mut bool_hit = false;

    //TODO figure out better way to init this
    let mut bar_time: f32 = 100000.0;
    let mut bar_inc: f32 = 100000.0;

    self.all_objs.borrow_mut().clear();

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
                    self.all_objs.borrow_mut().push(MapObject{time: bar_time as i32, class: 2, data: String::from(""), ..Default::default()});
                  }
                
                  //use uninherited point properties to calculate barline times
                  if map_obj.uninherited == 1 {
                    //set barline counter
                    bar_time = map_obj.time as f32;
                    bar_inc = map_obj.beatlength * map_obj.meter as f32;

                    //add current barline if not skipping barline (skip if effects is set to 8)
                    if map_obj.effects & 8 != 8 {
                      self.all_objs.borrow_mut().push(MapObject{time: bar_time as i32, class: 2, data: String::from(""), ..Default::default()});
                    }
                  }

                  //add timing point
                  self.all_objs.borrow_mut().push(map_obj);
                }
              } else if bool_hit {
                if let Ok(map_obj) = create_map_object(s, false) {
                  self.all_objs.borrow_mut().push(map_obj);
                }

                //if let Ok(hit_time) = s.split(",").nth(2).unwrap().parse::<i32>() {
                //  self.all_objs.borrow_mut().push(MapObject{time: hit_time, class: 3, data: String::from("")});
                //}
              } else {
                continue;
              }
            },
          }
        }
      }
    }
    self.all_objs.borrow_mut().sort_by_key(|k| k.time);
    self.status.set_text(0, &format!("editing {}", Path::new(&filename).file_name().unwrap().to_str().unwrap()));
  }

  fn open_file_browser(&self) {
    if let Ok(d) = env::current_dir() {
      if let Some(d) = d.to_str() {
        self.file_dialog.set_default_folder(d).expect("[brow] failed to set default folder");
      }
    }
  
    if self.file_dialog.run(Some(&self.window)) {
      self.in_filename.set_text("");
      if let Ok(directory) = self.file_dialog.get_selected_item() {
        let dir = directory.into_string().unwrap();
        self.in_filename.set_text(&dir);
        self.fill_out_filename();
        self.load_file();
      }
    }
  }

  fn write_output_points(&self) -> Result<()> {
    //sort new objects in chronological order
    //TODO inherited vs uninherited should not apply here, these are the new points
    self.new_objs.borrow_mut().sort_by_key(|k| (k.time, k.uninherited));

    //build up a vector with all old and new points sorted in chronological, then uninherited > inherited order
    let mut out_objs: Vec<MapObject> = Vec::new();
    out_objs.extend(self.new_objs.borrow().iter().cloned());
    for obj in self.all_objs.borrow().iter() {
      //uninherited/inherited lines
      if obj.class == 0 || obj.class == 1 {
        out_objs.push(obj.clone());
      }
    }
    out_objs.sort_by_key(|k| (k.time, k.uninherited));
    out_objs.dedup_by_key(|k| (k.time, k.uninherited));

    //write out new file
    let in_filename = self.in_filename.text();
    let out_filename = self.out_filename.text();

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
              if self.preview_check.check_state() == nwg::CheckBoxState::Checked && s_tokens.len() == 2 && s_tokens[0] == "Version" {
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

    MapObject{class: uninherited, time: time, beatlength: beatlength, meter: meter, sampleset: sampleset, sampleindex: sampleindex, volume: volume, uninherited: uninherited, effects: effects, data: p}
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

fn read_lines<P>(full_path: P) -> io::Result<io::Lines<io::BufReader<File>>> where P: AsRef<Path>, {
  let file = File::open(full_path)?;
  Ok(io::BufReader::new(file).lines())
}

fn main() {
  let icon_bytes = include_bytes!("../assets/svt.ico");
  nwg::init().expect("[main] failed to init nwg");

  //use Segoe UI with 16 size as default font
  let mut font = nwg::Font::default();

  nwg::Font::builder()
    .family("Segoe UI")
    .size(16)
    .build(&mut font).ok();
  nwg::Font::set_global_default(Some(font));

  //set icon on taskbar and on window top left
  let mut icon = nwg::Icon::default();
  let _res_ = nwg::Icon::builder()
    .source_bin(Some(icon_bytes))
    .strict(true)
    .build(&mut icon);

  let app = SVT::build_ui(Default::default()).expect("[main] failed to build UI");
  app.window.set_icon(Some(&icon));

  nwg::dispatch_thread_events();
}