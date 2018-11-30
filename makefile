image:
	cargo build --release 
	docker build -t greenhouse-rs .

test_data:
	mkdir -p test_data/test_bloomgc

test: test_data
	cargo test -- --nocapture

clean:
	rm -rf test_data
	cargo clean
	
