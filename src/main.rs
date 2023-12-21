use std::{thread, time::SystemTime};
use rand::distributions::{Uniform, Distribution};
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::fs::File;
use std::io::prelude::*;

//----------------------------------------------------------------------------//

const SIZE: usize = 50 + 2; // With borders! So actually 2 less
const GENS: usize = 10;
const ITERATIONS: usize = 2;
const MINIMUM_DENSITY: f64 = 0.6;
const MAXIMUM_DENSITY: f64 = 0.75;
const THREADS: usize = 8;

const RULES: usize = 512; // 2^9 because there are from 0 to 8 neighbors
const END: usize = SIZE - 1;
const CHUNK_SIZE: usize = RULES / THREADS;
const CELLS: usize = (SIZE - 2)*(SIZE - 2);
const CELLS_OVER_GENS: f64 = (CELLS * GENS) as f64;

//----------------------------------------------------------------------------//

#[inline]
fn get(matrix: &Vec<bool>, x: usize, y: usize) -> bool {
   matrix[y * SIZE + x]
}

#[inline]
fn set(matrix: &mut Vec<bool>, x: usize, y: usize, value: bool) {
   matrix[y * SIZE + x] = value;
}

#[inline]
fn apply_rule(rule: usize, neighbors: u16) -> bool {
   rule & (1 << neighbors) != 0
}

fn create_matrix(percentage: usize, rng: &mut Xoshiro256PlusPlus) -> Vec<bool> {
   let mut matrix = vec![false; SIZE*SIZE];

   // Place border corners
   set(&mut matrix,   0,   0, true);
   set(&mut matrix, END, END, true);
   set(&mut matrix,   0, END, true);
   set(&mut matrix, END,   0, true);

   // And border walls
   for i in 1..SIZE - 1 {
      set(&mut matrix,   i,   0, true);
      set(&mut matrix,   0,   i, true);
      set(&mut matrix,   i, END, true);
      set(&mut matrix, END,   i, true);
   }

   // Distribution skips top and bottom border, but not the sides unfortunately
   let between = Uniform::from(SIZE..SIZE*(SIZE - 1));
   let to_place = (percentage * CELLS)/100;
   let mut placed = 0;

   while placed < to_place {
      let i = between.sample(rng);
      
      placed += (!matrix[i]) as usize;
      matrix[i] = true;
   }

   matrix
}

fn automata(
   matrix: &Vec<bool>,
   new_matrix: &mut Vec<bool>,
   survive_rule: usize,
   birth_rule: usize
) -> f64
{
   let mut density = 0;

   for y in 1..SIZE - 1 {
      // If we keep past amounts we avoid checking all 8 cells
      let mut left_col = 3; // Left column at x = 1 is always 1 so 1x3
      let mut center   = get(matrix, 1, y)     as u16;
      let mut up_down  = get(matrix, 1, y + 1) as u16
                       + get(matrix, 1, y - 1) as u16;

      for x in 1..SIZE - 1 {
         let rule = if center > 0 { survive_rule } else { birth_rule };
         let neighbors = left_col + up_down;

         // Left column becomes current center column
         left_col = center + up_down;

         // Center column becomes right column
         center  = get(matrix, x + 1, y    ) as u16;
         up_down = get(matrix, x + 1, y - 1) as u16
                 + get(matrix, x + 1, y + 1) as u16;

         // Neighbors (left and center columns) + right column
         let new_value = apply_rule(rule, neighbors + center + up_down);

         set(new_matrix, x, y, new_value);

         // Density is captured as it goes to avoid another loop
         density += new_value as usize;
      }
   }

   density as f64
}

//----------------------------------------------------------------------------//

#[derive(Copy, Clone)]
struct Range {
   start: i8,
   size: i8,
}

impl Range {
   #[inline]
   fn reset_and_store(&mut self, other: &mut Range) {
      if self.size > other.size {
         other.start = self.start;
         other.size = self.size;
      }

      self.reset();
   }

   #[inline]
   fn add(&mut self, index: i8) {
      if self.start >= 0 {
         self.size += 1;
      }
      else {
         self.start = index;
         self.size = 0;
      }
   }

   #[inline]
   fn is_valid(&self) -> bool {
      self.start >= 0
   }

   #[inline]
   fn reset(&mut self) {
      self.start = -1;
      self.size = -1;
   }

   #[inline]
   fn get_end(&self) -> i8 {
      self.start + self.size
   }
}

impl Default for Range {
   fn default() -> Self {
      Self { start: -1, size: -1 }
   }
}

//----------------------------------------------------------------------------//

fn thread_main(id: usize) -> Vec<String> {
   let mut rng   = Xoshiro256PlusPlus::from_entropy();
   let mut rules = vec![Vec::new(); ITERATIONS];

   let mut best_noise_range = [Range::default(); ITERATIONS];
   let mut noise_range      = [Range::default(); ITERATIONS];
   let mut densities        = [0.0; ITERATIONS];

   for survive_rule in CHUNK_SIZE*id..CHUNK_SIZE*(id + 1) {
      println!("{}", survive_rule);
   
      for birth_rule in 0..RULES {
         for noise in 0..=100 {
            for _ in 0..GENS {
               let mut matrix = create_matrix(noise, &mut rng);

               for i in 0..ITERATIONS {
                  let mut new_matrix = matrix.clone();

                  densities[i] += automata(
                     &matrix, &mut new_matrix,
                     survive_rule, birth_rule
                  );

                  matrix = new_matrix;
               }
            }

            // Calculate the average density over GENS for each
            // iteration, updating the noise limits accordingly
            for i in 0..ITERATIONS {
               let density = densities[i] / CELLS_OVER_GENS;

               densities[i] = 0.0;

               if density >= MINIMUM_DENSITY && density <= MAXIMUM_DENSITY {
                  noise_range[i].add(noise as i8);
               }
               else {
                  noise_range[i].reset_and_store(&mut best_noise_range[i]);
               }
            }
         }

         // Push valid rules out
         for i in 0..ITERATIONS {
            noise_range[i].reset_and_store(&mut best_noise_range[i]);

            if best_noise_range[i].is_valid() {
               rules[i].push(format!(
                  "{},{},{},{}",
                  best_noise_range[i].start as u16,
                  best_noise_range[i].get_end() as u16,
                  birth_rule as u16,
                  survive_rule as u16
               ));
            }

            best_noise_range[i].reset();
         }
      }
   }

   let mut out = Vec::new();

   for i in 0..ITERATIONS {
      out.push(rules[i].join(","));
   }

   out
}

//----------------------------------------------------------------------------//

fn main() -> std::io::Result<()> {
   let mut threads = vec![];
   let t = SystemTime::now();

   for id in 0..THREADS {
      threads.push(thread::spawn(move || thread_main(id)));
   }

   let mut rules: Vec<String> = Vec::new();

   for thread in threads {
      rules.append(&mut thread.join().unwrap());
   }

   println!("took {0}ms", t.elapsed().unwrap().as_millis());

   for i in 0..ITERATIONS {
      let mut file = File::create(format!("rules{}.txt", i + 1))?;

      for j in (i..rules.len()).step_by(ITERATIONS) {
         file.write_all(rules[j].as_bytes())?;
         file.write_all(b",")?;
      }
   }

   Ok(())
}