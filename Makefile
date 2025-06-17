include $(TOPDIR)/rules.mk

PKG_NAME:=rust-eco
PKG_VERSION:=0.1.0
PKG_RELEASE:=1

PKG_SOURCE:=$(PKG_NAME)-$(PKG_VERSION).tar.gz
PKG_SOURCE_URL=https://github.com/anthropics/rust-eco/releases/download/v$(PKG_VERSION)
PKG_HASH:=skip

PKG_MAINTAINER:=Claude Code <noreply@anthropic.com>
PKG_LICENSE:=MIT
PKG_LICENSE_FILES:=LICENSE

PKG_BUILD_DEPENDS:=rust/host

include $(INCLUDE_DIR)/package.mk
include $(INCLUDE_DIR)/rust.mk

define Package/rust-eco
  TITLE:=A Rust async runtime inspired by lua-eco
  SECTION:=lang
  CATEGORY:=Languages
  SUBMENU:=Rust
  URL:=https://github.com/anthropics/rust-eco
  DEPENDS:=+libc +libpthread +librt
endef

define Package/rust-eco/description
  rust-eco is a Rust async runtime inspired by lua-eco, providing
  lightweight coroutines and built-in modules for high-performance applications.
  Features async runtime, file I/O, networking, protocols, system utilities,
  synchronization, and structured logging.
endef

define Package/rust-eco/Module
  TITLE:=$1 support for rust-eco
  SECTION:=lang
  CATEGORY:=Languages
  SUBMENU:=Rust
  URL:=https://github.com/anthropics/rust-eco
  DEPENDS:=+rust-eco $2
endef

Package/rust-eco-runtime=$(call Package/rust-eco/Module,async runtime)
Package/rust-eco-file=$(call Package/rust-eco/Module,file I/O)
Package/rust-eco-socket=$(call Package/rust-eco/Module,socket networking)
Package/rust-eco-http=$(call Package/rust-eco/Module,HTTP client/server,+rust-eco-socket)
Package/rust-eco-protocols=$(call Package/rust-eco/Module,MQTT/WebSocket,+rust-eco-socket +rust-eco-http)
Package/rust-eco-sync=$(call Package/rust-eco/Module,synchronization primitives)
Package/rust-eco-channel=$(call Package/rust-eco/Module,async channels)
Package/rust-eco-dns=$(call Package/rust-eco/Module,DNS resolver,+rust-eco-socket)
Package/rust-eco-sys=$(call Package/rust-eco/Module,system utilities)
Package/rust-eco-log=$(call Package/rust-eco/Module,structured logging)
Package/rust-eco-time=$(call Package/rust-eco/Module,time utilities)

# Rust build configuration
CARGO_VARS += \
	CARGO_HOME="$(CARGO_HOME)" \
	RUSTFLAGS="$(RUSTFLAGS)"

RUST_PKG_FEATURES:=default

define Build/Compile
	$(call Build/Compile/Cargo,rust-eco,--release --bin eco --lib)
endef

define Package/rust-eco/install
	$(INSTALL_DIR) $(1)/usr/bin $(1)/usr/lib
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/target/$(RUST_TARGET_TRIPLE)/release/eco $(1)/usr/bin/rust-eco
	$(CP) $(PKG_BUILD_DIR)/target/$(RUST_TARGET_TRIPLE)/release/deps/librust_eco*.so $(1)/usr/lib/ 2>/dev/null || true
endef

define Package/rust-eco-runtime/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Runtime module is part of core library
endef

define Package/rust-eco-file/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# File module is part of core library
endef

define Package/rust-eco-socket/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Socket module is part of core library
endef

define Package/rust-eco-http/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# HTTP module is part of core library
endef

define Package/rust-eco-protocols/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Protocols module is part of core library
endef

define Package/rust-eco-sync/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Sync module is part of core library
endef

define Package/rust-eco-channel/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Channel module is part of core library
endef

define Package/rust-eco-dns/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# DNS module is part of core library
endef

define Package/rust-eco-sys/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Sys module is part of core library
endef

define Package/rust-eco-log/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Log module is part of core library
endef

define Package/rust-eco-time/install
	$(INSTALL_DIR) $(1)/usr/lib/rust-eco/modules
	# Time module is part of core library
endef

$(eval $(call BuildPackage,rust-eco))
$(eval $(call BuildPackage,rust-eco-runtime))
$(eval $(call BuildPackage,rust-eco-file))
$(eval $(call BuildPackage,rust-eco-socket))
$(eval $(call BuildPackage,rust-eco-http))
$(eval $(call BuildPackage,rust-eco-protocols))
$(eval $(call BuildPackage,rust-eco-sync))
$(eval $(call BuildPackage,rust-eco-channel))
$(eval $(call BuildPackage,rust-eco-dns))
$(eval $(call BuildPackage,rust-eco-sys))
$(eval $(call BuildPackage,rust-eco-log))
$(eval $(call BuildPackage,rust-eco-time))