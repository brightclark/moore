module A;
	bit a0;
	int a1;
	bit [41:0] a2;

	logic b0;
	integer b1;
	logic [41:0] b2;

	// enum {
	// 	Foo, Bar
	// } c0;
	// enum int {
	// 	Baz, Buz
	// } c1;
	struct {
		logic a;
		int b;
		struct {
			bit x;
			integer y;
		} c;
	} c2;
endmodule

//@ elab A
//| entity @A () () {
//|     %a0 = sig i1
//|     %a1 = sig i32
//|     %a2 = sig i42
//|     %b0 = sig i1
//|     %b1 = sig i32
//|     %b2 = sig i42
//|     %c2 = sig {i1, i32, {i1, i32}}
//| }
