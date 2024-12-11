build:
	cargo build --release --target=thumbv8m.main-none-eabihf
	
flash:
	sudo picotool load -u -v -x -t elf target/thumbv8m.main-none-eabihf/release/tetris

add-target:
	rustup target add thumbv8m.main-none-eabihf

clean:
	cargo clean

.PHONY: build flash clean
