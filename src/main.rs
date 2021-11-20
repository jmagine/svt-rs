#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;
use std::{cell::RefCell};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

#[derive(Default, NwgUi)]
pub struct SVT {
  //TODO make this pixel-perfect pretty
  #[nwg_control(size: (300, 300), position: (500, 500), title: "SVT", accept_files: true, flags: "WINDOW|VISIBLE")]
  #[nwg_events( OnWindowClose: [SVT::close_window], OnFileDrop: [SVT::drop_file(SELF, EVT_DATA)] )]
  window: nwg::Window,

  //timing point input
  #[nwg_control(text: "", size: (290, 50), position: (5,5), flags: "VISIBLE|AUTOVSCROLL|TAB_STOP")]
  inherited_text: nwg::TextBox,

  #[nwg_control(text: "Apply to:", size: (95, 20), position: (5, 60))]
  apply_label: nwg::Label,

  //toggles note changes
  #[nwg_control(text: "Hits", size: (95, 20), position: (5, 80), check_state: CheckBoxState::Checked)]
  note_check: nwg::CheckBox,

  //toggles barline changes
  #[nwg_control(text: "Barlines", size: (95, 20), position: (5, 100), check_state: CheckBoxState::Checked)]
  barline_check: nwg::CheckBox,

  //toggles inh line changes
  #[nwg_control(text: "Inherited lines", size: (95, 20), position: (5, 120), check_state: CheckBoxState::Unchecked)]
  inh_check: nwg::CheckBox,

  //offset time
  #[nwg_control(text: "0", size: (20, 20), position: (105, 80))]
  offset_text: nwg::TextInput,

  #[nwg_control(text: "offset", size: (95, 20), position: (130, 82))]
  offset_label: nwg::Label,

  //buffer time
  #[nwg_control(text: "3", size: (20, 20), position: (105, 105))]
  buffer_text: nwg::TextInput,

  #[nwg_control(text: "buffer", size: (95, 20), position: (130, 107))]
  buffer_label: nwg::Label,

  //input map filename
  #[nwg_control(text: "", size: (175, 23), position: (6, 186))]
  in_filename: nwg::TextInput,

  //select map button
  #[nwg_control(text: "Select .osu File", size: (110, 25), position: (185, 185))]
  #[nwg_events( OnButtonClick: [SVT::open_file] )]
  open_button: nwg::Button,

  //output map filename
  #[nwg_control(text: "", size: (175, 23), position: (6, 216))]
  out_filename: nwg::TextInput,

  //toggles preview
  #[nwg_control(text: "Preview changes", size: (110, 25), position: (185, 215), check_state: CheckBoxState::Checked)]
  #[nwg_events(OnButtonClick: [SVT::fill_out_filename])]
  preview_check: nwg::CheckBox,
  
  //place apply button near bottom
  #[nwg_control(text: "Apply", size: (290, 25), position: (5, 245))]
  #[nwg_events( OnButtonClick: [SVT::apply_sv] )]
  hello_button: nwg::Button,

  //place status bar at the very bottom
  #[nwg_control(text: "no map selected")]
  status: nwg::StatusBar,

  //open file dialog
  #[nwg_resource(title: "Open File", action: nwg::FileDialogAction::Open, filters: "osu(*.osu)")]
  file_dialog: nwg::FileDialog,

  all_points: RefCell<Vec<String>>,
  inh_points: RefCell<Vec<String>>,
  //inh_times: RefCell<Vec<i32>>,
  hit_times: RefCell<Vec<i32>>,
  bar_times: RefCell<Vec<i32>>,

  new_points: RefCell<Vec<String>>,
}

impl SVT {
  fn apply_sv(&self) {
    self.load_file();

    let cmd = self.inherited_text.text();
    let mut lines = cmd.lines();
    let start_line = lines.next();
    let end_line = lines.next();

    match (start_line, end_line) {
      (Some(_), Some(_)) => {},
      _ => {
        println!("formatting issue: {}", cmd);
        return;
      },
    }

    let start_tokens: Vec<&str> = start_line.unwrap().split(",").collect();
    let end_tokens: Vec<&str> = end_line.unwrap().split(",").collect();

    if start_tokens.len() != 8 || end_tokens.len() != 8 {
      println!("formatting issue: {}", cmd);
      return;
    }
    let t_offset = self.offset_text.text().parse::<i32>();
    let t_buffer = self.buffer_text.text().parse::<i32>();

    let start_time = start_tokens[0].parse::<i32>();
    let start_bl = start_tokens[1].parse::<f32>();
    let start_vol = start_tokens[5].parse::<i32>();

    let end_time = end_tokens[0].parse::<i32>();
    let end_bl = end_tokens[1].parse::<f32>();
    let end_vol = end_tokens[5].parse::<i32>();
    //  println!("[apply] need exactly 2 timing points for start/end");
    //  return;

    match (t_offset, t_buffer, start_time, start_bl, start_vol, end_time, end_bl, end_vol) {
      (Ok(t_off), Ok(t_buf), Ok(s_t), Ok(s_b), Ok(s_v), Ok(e_t), Ok(e_b), Ok(e_v)) => {
        let s_s = -100.0 / s_b;
        let e_s = -100.0 / e_b;
        println!("[apply] t:{}->{} sv:{}->{} vol:{}->{}", s_t, e_t, s_s, e_s, s_v, e_v);

        //validation
        if s_t > e_t {println!("[apply] start time should be less than end time"); return;}
        if s_s <= 0.0 || e_s <= 0.0 {println!("[apply] sv values should be positive (neg beatlength values"); return;} 
        if s_v < 0 || s_v > 100 || e_v < 0 || e_v > 100 {println!("[apply] volumes should be within [0, 100]"); return;}
        
        //compute change per time tick
        let t_diff = e_t - s_t;
        let s_diff = e_s - s_s;
        let v_diff = e_v - s_v;
        let s_per_ms = s_diff / t_diff as f32;
        let v_per_ms = v_diff as f32 / t_diff as f32;

        self.new_points.borrow_mut().clear();

        //TODO insert a match on interp mode to determine whether to SV notes or lines

        //to apply changes to inherited points, iterate over all inh_points
        for l in self.inh_points.borrow().iter() {
          //no error validation needed, done when reading file
          let inh_tokens: Vec<&str> = l.split(",").collect();
          
          let time = inh_tokens[0].parse::<i32>();
          match time {
            Ok(t) => {
              //time inside range
              if t >= s_t - t_buf && t <= e_t + t_buf {
                let new_t = t + t_off;
                let new_s = s_s + (t - s_t) as f32 * s_per_ms;
                let new_b = -100.0 / new_s;
                let new_v = (s_v as f32 + (t - s_t) as f32 * v_per_ms) as u32;
                let new_point = format!("{},{},{},{},{},{},{},{}", new_t, new_b, inh_tokens[2], inh_tokens[3], inh_tokens[4], new_v, inh_tokens[6], inh_tokens[7]);
                println!("[new] {}", new_point);
                self.new_points.borrow_mut().push(new_point);
              }
            },
            _ => {
              println!("[apply] invalid time {}", l);
            }
          }
        }

        let mut all_times: Vec<i32> = Vec::new();

        if self.barline_check.check_state() == nwg::CheckBoxState::Checked {
          all_times.extend(self.bar_times.borrow().iter().cloned());
        }

        if self.note_check.check_state() == nwg::CheckBoxState::Checked {
          all_times.extend(self.hit_times.borrow().iter().cloned());
        }
        all_times.sort();

        //apply to applicable times
        for t in all_times.iter() {
          if t >= &(s_t - t_buf) && t <= &(e_t + t_buf) {
            let new_t = t + t_off;
            let new_s = s_s + (t - s_t) as f32 * s_per_ms;
            let new_b = -100.0 / new_s;
            let new_v = (s_v as f32 + (t - s_t) as f32 * v_per_ms) as u32;

            let new_point = format!("{},{},{},{},{},{},{},{}", new_t, new_b, start_tokens[2], start_tokens[3], start_tokens[4], new_v, start_tokens[6], start_tokens[7]);
            println!("[new] {}", new_point);

            self.new_points.borrow_mut().push(new_point);
          }
        }
      },
      _ => {
        println!("issue: {}", cmd);
        return;
      },
    }

    //merge new points into old ones - delete old point if new one is identical
    let mut output_points: Vec<String> = Vec::new();
    output_points.extend(self.new_points.borrow().iter().cloned());
    output_points.extend(self.all_points.borrow().iter().cloned());
    output_points.sort_by_key(|k| (k.split(",").nth(0).unwrap().parse::<i32>().unwrap(), k.split(",").nth(6).unwrap().parse::<i32>().unwrap()));
    output_points.dedup_by_key(|k| (k.split(",").nth(0).unwrap().parse::<i32>().unwrap(), k.split(",").nth(6).unwrap().parse::<i32>().unwrap()));
    //println!("{:?}", output_points);
    //all output is same except for timing points section

    //write out new file
    let in_filename = self.in_filename.text();
    let out_filename = self.out_filename.text();

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
              for o_point in output_points.iter() {
                out_string += &o_point;
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
    }
    let mut out_file = File::create(out_filename).unwrap();
    let _ = write!(&mut out_file, "{}", out_string);
  }
  
  fn close_window(&self) {
    nwg::stop_thread_dispatch();
  }

  fn open_file(&self) {
    if let Ok(d) = env::current_dir() {
      if let Some(d) = d.to_str() {
        self.file_dialog.set_default_folder(d).expect("Failed to set default folder.");
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

  pub fn drop_file(&self, data: &nwg::EventData) {
    self.in_filename.set_text(&data.on_file_drop().files().pop().unwrap());
    self.fill_out_filename();
    self.load_file();
  }

  pub fn fill_out_filename(&self) {
    if self.preview_check.check_state() == nwg::CheckBoxState::Checked {
      let in_filename = &self.in_filename.text();
      let folder = Path::new(in_filename).parent().unwrap();
      self.out_filename.set_text(&format!("{}/{}", folder.to_str().unwrap(), "preview.osu"));
    } else {
      self.out_filename.set_text(&self.in_filename.text());
    }
  }

  // load and parse .osu file line by line
  pub fn load_file(&self) {
    let filename = self.in_filename.text();

    // determine filename and extension
    let ext = Path::new(&filename).extension();

    let folder = String::from(Path::new(&filename).parent().unwrap().to_str().unwrap());
    println!("[load] folder: {}", folder);

    // skip any file that is not .osu
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
    //self.inh_points = Vec::new();
    //self.inh_times = Vec::new();
    //self.hit_times = Vec::new();

    self.all_points.borrow_mut().clear();
    self.inh_points.borrow_mut().clear();
    self.hit_times.borrow_mut().clear();
    self.bar_times.borrow_mut().clear();


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
                //println!("[load] timing {}", s);
                //TODO do barlines when processing sv section and then process the hit objects vec + barlines vec after filereading is complete
                
                
                //if let Ok(time) = s.split(",").nth(0).unwrap().parse::<i32>() {
                //  println!("[load] sv {}", time);
                  //self.inh_times.borrow_mut().push(time);
                //}

                let time_tokens: Vec<&str> = s.split(",").collect();
                if time_tokens.len() != 8 {
                  continue;
                }

                let time = time_tokens[0].parse::<i32>();
                let beatlength = time_tokens[1].parse::<f32>();
                let meter = time_tokens[2].parse::<i32>();
                let uninherited = time_tokens[6].parse::<i32>();
                let effect = time_tokens[7].parse::<i32>();
                
                match (time, beatlength, meter, uninherited, effect) {
                  (Ok(t), Ok(bl), Ok(m), Ok(uninh), Ok(eff)) => {
                    //add barlines since last timing point

                    while bar_time + bar_inc < t as f32 {
                      bar_time += bar_inc;
                      self.bar_times.borrow_mut().push(bar_time as i32);
                    }

                    if uninh == 1 {
                      //uninherited point

                      //only skip barline if effect bit 3 is set
                      if eff & 8 != 8 {
                        self.bar_times.borrow_mut().push(t);
                      }
                      bar_time = t as f32;
                      bar_inc = bl * m as f32;
                    } else if uninh == 0 {
                      //inherited point
                      self.inh_points.borrow_mut().push(s.clone());
                    }
                    self.all_points.borrow_mut().push(s.clone());
                  },
                  _ => {
                    println!("[load] issue {}", s);
                    return;
                  },
                }
              } else if bool_hit {
                if let Ok(hit_time) = s.split(",").nth(2).unwrap().parse::<i32>() {
                  self.hit_times.borrow_mut().push(hit_time);
                }
              } else {
                continue;
              }
            },
          }
        }
      }
    }
    self.status.set_text(0, &format!("editing {}", Path::new(&filename).file_name().unwrap().to_str().unwrap()));
  }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(full_path: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
  let file = File::open(full_path)?;
  Ok(io::BufReader::new(file).lines())
}

fn main() {
  nwg::init().expect("Failed to init Native Windows GUI");

  // use Segoe UI with 16 size as default font
  let mut font = nwg::Font::default();

  nwg::Font::builder()
    .family("Segoe UI")
    .size(16)
    .build(&mut font).ok();
  nwg::Font::set_global_default(Some(font));

  let _app = SVT::build_ui(Default::default()).expect("Failed to build UI");

  nwg::dispatch_thread_events();
}