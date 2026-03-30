# sublate

A tool for understanding how instructions are built.

It's a little workspace for taking programmatic ideas, breaking them down into steps, and seeing how they actually move.

Currently, it's a rust-based dioxus project that uses a python vm to run in the browser. Behind the scenes that python
code binds to hegel to test things exhaustively, roughr to display things for understanding, and rustpython to execute
interactively. it's in early development, focused on building a bridge between understanding functions, how they're
tested, common errors caught by mistakes, and visual intuition.

