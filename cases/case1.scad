module example_intersection()
{
  // Options for case
  thickness = 6;
  angle = 20;
  bezel = 15;
  base_depth = 50;
  cable_notch_width = 30;

  // Physical params of display
  screen_width = 163;
  screen_height = 97;
  inset = 4;
  bottom_inset = 10;
  inset_thickness = 2;

  // Computed params
  frame_outside_width = screen_width + 2 * bezel;
  frame_outside_height = screen_height + 2 * bezel;

  leftover_inset = bezel - inset;
  leftover_bottom_inset = bezel - bottom_inset;

  rotate([0,0,0]) {
    // Main frame
    difference() {
      rotate([angle, 0, 0]) {
        difference() {
          cube([frame_outside_width, frame_outside_height, thickness], center=false);

          // All the way through hole
          translate([bezel,bezel,0]) {
            cube([screen_width, screen_height, thickness], center= false);
          };

          // Inset for screen only goes half way
          translate([leftover_inset, leftover_bottom_inset, inset_thickness]) {
            cube([screen_width + 2 * inset, screen_height + bottom_inset + inset, thickness-inset_thickness], center = false);
          };
        };

        // Add a back for the little slot
        difference() {
          translate([0, leftover_bottom_inset, thickness - inset_thickness]) {
            cube([frame_outside_width, bottom_inset, inset_thickness], center = false);
          };

          // Remove a notch for the cable:
          translate([(frame_outside_width - cable_notch_width) / 2, leftover_bottom_inset, thickness - inset_thickness]) {
            cube([cable_notch_width, bottom_inset, inset_thickness], center = false);
          };
        };
      };

      // strip the lip off the bottom:
      void_height = 10;
      void_depth = 40;
      translate([0, void_height * -1, void_depth / 2 * -1]) {
        cube([frame_outside_width, void_height, void_depth], center=false);
      }
    }

    // Base
    difference() {
      cube([frame_outside_width, thickness, base_depth]);

      translate([thickness, 0, 0]) {
        cube([frame_outside_width - thickness * 2, thickness, base_depth]);
      }

      // strip the lip off the front:
      translate([0, 0, thickness * -1]) {
        rotate([angle, 0, 0]) {
          cube([frame_outside_width, frame_outside_height, thickness], center=false);
        }
      }
    }
  }
}

example_intersection();