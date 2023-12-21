# Cellular Automata Density

Given a matrix size and number of iterations, this code calculates the longest initial noise range for all 2<sup>10</sup> possible configurations of a standard cellular automaton in order to achieve a desired final density range.

## Definitions

A standard cellular automaton as defined here is an automaton which just checks the 8 neighbors around it, as does Conway's game of life. Such an automaton can be expressed in two rules: a survive rule, which decides if an alive cell keeps living, and a birth rule, which decides if a dead cell becomes alive.

To generalize this concept, each rule can be a hash map having numbers from 0 to 8. Then, during the iteration, it is seen if the amount of neighbors of a cell is found in such a hash map. A survive rule represented by the hash map {0, 2, 6, 8}, for example, makes it so that a cell will only keep living if the amount of neighbors around it is even, except four.

Thus, there are 2<sup>10</sup> possible configurations for this kind of cellular automaton: each number from 0 to 8 is or is not in the hash map, so there are 2<sup>9</sup> possibilities per hash map and two hash maps (survive and birth).