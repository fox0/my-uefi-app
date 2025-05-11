SRC=$(shell find src -name '*.rs')

run: esp/efi/boot/bootx64.efi
	qemu-system-x86_64 \
		-enable-kvm \
		-machine q35,i8042=on \
		-drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
		-drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_VARS.fd \
		-drive format=raw,file=fat:rw:esp

esp/efi/boot/bootx64.efi: target/x86_64-unknown-uefi/debug/my-uefi-app.efi
	cp $< $@

target/x86_64-unknown-uefi/debug/my-uefi-app.efi: Cargo.toml ${SRC}
	cargo build

clean:
	cargo clean
	rm -rf esp/efi/boot/bootx64.efi
