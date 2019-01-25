ifeq ($(shell uname),Darwin)
    LDFLAGS := -Wl,-dead_strip
else
    LDFLAGS := -Wl,--gc-sections -lpthread -ldl -lm
endif

all: target/cdatastructures
	target/cdatastructures

target:
	mkdir -p $@

target/cdatastructures: target/main.o ../../../target/release/libcdatastructures.a
	$(CC) -o $@ $^ $(LDFLAGS)

target/release/libcdatastructures.a: src/lib.rs Cargo.toml
	cargo build --release

target/main.o: src/example.c | target
	$(CC) -o $@ -c $<

clean:
	rm -rf target
