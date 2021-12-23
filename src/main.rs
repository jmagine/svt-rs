#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate native_windows_gui as nwg;
use nwg::NativeUi;

mod ui;
mod svt;

fn main() {
  nwg::init().expect("[main] failed to init nwg");

  //use Segoe UI with 16 size as default font
  let mut font = nwg::Font::default();
  nwg::Font::builder()
    .family("Segoe UI")
    .size(16)
    .build(&mut font).ok();
  nwg::Font::set_global_default(Some(font));

  //start the nwg app
  let app = ui::UI::build_ui(Default::default()).expect("[main] failed to build UI");
  app.init();

  //initialize tooltips based off application control elements
  let mut tooltip = nwg::Tooltip::default();

  //TODO could maybe use init to register the tooltip fields
  let _res_ = nwg::Tooltip::builder()
    .register(&app.inherited_text, "Paste timing point start/end pair(s) here. Copy/paste from timing panel. These timing points contain the start/end times, SVs, and volumes which are interpolated for the selected objects. (Example format: 111376,-76.92308,4,1,0,100,0,1)")
    .register(&app.lin_sv_check, "Change slider velocity linearly for selected objects (hits/barlines/inh. lines)")
    .register(&app.exp_sv_check, "Change slider velocity exponentially for selected objects (hits/barlines/inh. lines)")
    .register(&app.pol_sv_check, "Change slider velocity polynomially using exp. factor for selected objects (hits/barlines/inh. lines)")
    .register(&app.flat_sv_check, "Flat SV change for selected inh. lines")
    .register(&app.vol_check, "Change volume smoothly for selected objects (hits/barlines/inh. lines)")
    .register(&app.hit_check, "Change hitobjects (notes, spinners, sliders) between start/end points")
    .register(&app.barline_check, "Change barlines (big white lines) between start/end points")
    .register(&app.inh_check, "Change current inherited lines between start/end points")
    .register(&app.offset_label, "(integer) Place new timing points at offset (in ms) from map object (negative offset for before, positive for after)")
    .register(&app.buffer_label, "(integer) Include map objects (in ms) before and after the start/end timing points, useful if objects are not perfectly snapped")
    .register(&app.pol_exp_label, "(decimal) Exponent for polynomial SV. Recommended values are [0.5, 1) for slowdowns and (1.0, 2.0] for speedups. Applied following a (sv_diff) * (t / t_diff)^exp curve")
    .register(&app.flat_sv_label, "(decimal) Amount of SV change to apply to each inherited line")
    .register(&app.ign_bpm_check, "End timing point SV is normally relative to end timing point BPM, but if checked, can be made relative to start timing point BPM")
    .register(&app.open_button, "Select map to change")
    .register(&app.in_filename, "Map being edited")
    .register(&app.out_filename, "Output location")
    .register(&app.preview_check, "If enabled, creates a preview diff alongside your current diff which shows how the changes would potentially look without touching the original")
    .register(&app.apply_button, "Apply SV/vol changes")
    .register(&app.undo_button, "Undo most recent change")
    .build(&mut tooltip);
  tooltip.set_delay_time(Some(50));

  //start handling events
  nwg::dispatch_thread_events();
}