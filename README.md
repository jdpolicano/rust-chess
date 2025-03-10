# Roadmap

I plan to use the chess crate to avoid having to devlop my own board representation and to handle move generation etc...this should save a lot of headaches up front.

1. I need a UCI compatible server that can sit on top of my engine. This should make it easy to plug the engine into existing interfaces like lichess.
2. I would like to build a test engine that can sit between two versions of my engine and have them play a game with: time limited moves, swappings sides, and statistic collection (i.e., wins, losses, draws). The idea would be to have my own engine play itself over time to test its performance as I iterate.
3. I should create a PGN interface so I can export test evidence and review games myself to get insights into the chess engine's performance and to view games post-facto.
4. I need to create the engine itself that will take in moves, update its board, and then evaluate next moves and produce a result, given some set of options (i.e., max-depth)



# Engine Planning

What is the best way to incentise the engine to make good moves? I plan to use a minimax approach with prunning, but need to do some reasearch on a good baseline implementation for
an evalutation function.

## Resources
1. transpotion tables: https://www.chessprogramming.org/Transposition_Table#How_it_works
