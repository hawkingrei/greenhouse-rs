image:
	cargo build --release
	docker build -t greenhouse-rs .

test_data:
	mkdir test_data

test: test_data
	cargo test -- --nocapture

clean:
	rm -rf test_data
	
