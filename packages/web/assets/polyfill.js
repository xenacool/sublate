if(!console.createTask) console.createTask = function(n) { return { run: function(f) { return f(); } }; };
