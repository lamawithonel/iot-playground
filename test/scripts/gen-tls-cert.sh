#!/usr/bin/env bash
# Generate TLS certificates for local development and testing.
#
# Subcommands:
#   ca              Generate a root CA
#   server <name>   Generate a server certificate signed by the CA
#   client <name>   Generate a client certificate signed by the CA
#
# Defaults match AWS IoT requirements:
#   CA + server:  ECDSA P-256, SHA256
#   Client:       ECDSA P-384, SHA384
#
# Options (apply to all subcommands):
#   --curve <curve>   Override the elliptic curve (e.g., P-256, P-384)
#   --hash <hash>     Override the hash algorithm (e.g., SHA256, SHA384)
#   --force           Regenerate even if files already exist
#   --cn <cn>         Override the Common Name (default: platform-dependent)
#   --san <san>       Override the Subject Alt Name (default: platform-dependent)
#   --days <days>     Certificate validity in days (default: 3650)
#
# Default CN/SAN is determined by:
#   1. BROKER_HOST_IP environment variable (if set)
#   2. Platform detection: 127.0.0.1 on macOS, 192.168.1.1 on Linux
#
# Output is YAML-formatted for easy parsing by other scripts.
#
# NOTE: The tls:ca, tls:server, and tls:client tasks in .mise.toml
# have usage specs that mirror these arguments for tab-completion.
# If you add, remove, or rename arguments here, update the
# corresponding usage blocks in .mise.toml to match.

set -o errexit
set -o nounset
set -o pipefail

_CERT_DIR='.local/certs'
_KEY_DIR='.local/private'
_CSR_DIR='.cache/signing_requests'

_DEFAULT_CA_CN='IoT Playground Root CA'
_DEFAULT_DAYS='3650'

# Reject names that could traverse paths or contain shell metacharacters.
_validate_name() {
	if ! [[ "$1" =~ ^[A-Za-z0-9._-]+$ ]]; then
		echo "ERROR: invalid name '${1}' — must match [A-Za-z0-9._-]+" >&2
		exit 1
	fi
}

# Reject CN values that could inject X.509 subject fields.
# OpenSSL interprets '/' in -subj as a field separator.
# Control characters could inject arbitrary extensions via extfile.
_validate_cn() {
	if [ -z "$1" ]; then
		echo 'ERROR: CN must not be empty' >&2
		exit 1
	fi
	if [[ "$1" == */* ]]; then
		echo "ERROR: invalid CN '${1}' — must not contain '/'" >&2
		exit 1
	fi
	if [[ "$1" =~ [^[:print:]] ]]; then
		echo "ERROR: invalid CN — contains control characters" >&2
		exit 1
	fi
}

# Reject SAN values containing control characters.  Newlines in
# SAN flow into OpenSSL extfile via printf, where they would
# inject arbitrary X.509 extensions.
_validate_san() {
	if [ -z "$1" ]; then
		echo 'ERROR: SAN must not be empty' >&2
		exit 1
	fi
	if [[ "$1" =~ [^[:print:]] ]]; then
		echo "ERROR: invalid SAN — contains control characters" >&2
		exit 1
	fi
}

# Platform-aware CN/SAN defaults: env var → uname → fallback.
# Defaults are validated through the canonical _validate_cn /
# _validate_san functions to maintain a single validation path.
if [ -n "${BROKER_HOST_IP:-}" ]; then
	_DEFAULT_CN="$BROKER_HOST_IP"
	_DEFAULT_SAN="IP:${BROKER_HOST_IP}"
elif [ "$(uname -s)" = 'Darwin' ]; then
	_DEFAULT_CN='127.0.0.1'
	_DEFAULT_SAN='IP:127.0.0.1'
else
	_DEFAULT_CN='192.168.1.1'
	_DEFAULT_SAN='IP:192.168.1.1'
fi
_validate_cn "$_DEFAULT_CN"
_validate_san "$_DEFAULT_SAN"

# Curve names: OpenSSL uses the name form, not the P-xxx form.
# Map user-friendly names to OpenSSL names.
_map_curve() {
	case "$1" in
		P-256|p-256|prime256v1|secp256r1) echo 'prime256v1' ;;
		P-384|p-384|secp384r1) echo 'secp384r1' ;;
		P-521|p-521|secp521r1) echo 'secp521r1' ;;
		*) echo "$1" ;;
	esac
}

_map_hash() {
	case "$1" in
		SHA256|sha256) echo '-sha256' ;;
		SHA384|sha384) echo '-sha384' ;;
		SHA512|sha512) echo '-sha512' ;;
		*) echo "-${1}" ;;
	esac
}

_usage() {
	echo 'Usage: gen-tls-cert.sh <subcommand> [name] [options]'
	echo ''
	echo 'Subcommands:'
	echo '  ca              Generate root CA certificate'
	echo '  server <name>   Generate server certificate'
	echo '  client <name>   Generate client certificate'
	echo ''
	echo 'Options:'
	echo '  --curve <curve>   Elliptic curve (default: P-256 for'
	echo '                    CA/server, P-384 for client)'
	echo '  --hash <hash>     Hash algorithm (default: SHA256 for'
	echo '                    CA/server, SHA384 for client)'
	echo '  --force           Regenerate even if files exist'
	echo '  --cn <cn>         Common Name (default: platform-dependent;'
	echo '                    see BROKER_HOST_IP env var)'
	echo '  --san <san>       Subject Alt Name (default: platform-dependent;'
	echo '                    see BROKER_HOST_IP env var)'
	echo '  --days <days>     Validity in days (default: 3650)'
	exit 1
}

_parse_opts() {
	_force='false'
	_cn="$_DEFAULT_CN"
	_cn_explicit='false'
	_san="$_DEFAULT_SAN"
	_days="$_DEFAULT_DAYS"
	_curve=''
	_hash=''

	while [ $# -gt 0 ]; do
		case "$1" in
			--curve)
				if [ $# -lt 2 ]; then
					echo 'ERROR: --curve requires a value' >&2; _usage
				fi
				_curve="$2"; shift 2
				;;
			--hash)
				if [ $# -lt 2 ]; then
					echo 'ERROR: --hash requires a value' >&2; _usage
				fi
				_hash="$2"; shift 2
				;;
			--force) _force='true'; shift ;;
			--cn)
				if [ $# -lt 2 ]; then
					echo 'ERROR: --cn requires a value' >&2; _usage
				fi
				_validate_cn "$2"
				_cn="$2"; _cn_explicit='true'; shift 2
				;;
			--san)
				if [ $# -lt 2 ]; then
					echo 'ERROR: --san requires a value' >&2; _usage
				fi
				_validate_san "$2"
				_san="$2"; shift 2
				;;
			--days)
				if [ $# -lt 2 ]; then
					echo 'ERROR: --days requires a value' >&2; _usage
				fi
				_days="$2"; shift 2
				;;
			*) echo "Unknown option: $1" >&2; _usage ;;
		esac
	done
}

_ensure_dirs() {
	mkdir -p "$_CERT_DIR/ca" "$_CERT_DIR/server" "$_CERT_DIR/client"
	mkdir -p "$_KEY_DIR"
	mkdir -p "$_CSR_DIR"
	chmod 700 "$_KEY_DIR"
}

_emit_yaml() {
	local _type="$1" _name="$2" _cert="$3" _key="$4"
	local _curve_display="$5" _hash_display="$6"
	shift 6

	# Quote values for safe inclusion in YAML.  Uses YAML
	# single-quoted scalars, escaping single quotes by doubling.
	local _yaml_quote
	_yaml_quote() {
		local _s="${1//\'/\'\'}"
		printf "'%s'" "$_s"
	}

	local _yaml
	_yaml="type: $(_yaml_quote "$_type")
name: $(_yaml_quote "$_name")
ca_cert: $(_yaml_quote "${_CERT_DIR}/ca/root.crt")
cert: $(_yaml_quote "$_cert")
key: $(_yaml_quote "$_key")
algorithm: 'ecdsa'
curve: $(_yaml_quote "$_curve_display")
hash: $(_yaml_quote "$_hash_display")
cn: $(_yaml_quote "$_cn")"

	# SAN is not applicable to CA certs
	if [ "$_type" != 'ca' ]; then
		_yaml="${_yaml}
san: $(_yaml_quote "$_san")
csr: $(_yaml_quote "${_CSR_DIR}/${_name}.csr")"
	fi

	echo "$_yaml"

	# Save info.yaml alongside the cert
	local _info_dir
	_info_dir="$(dirname "$_cert")"
	echo "$_yaml" > "${_info_dir}/${_name}.info.yaml"
}

_gen_ca() {
	_parse_opts "$@"

	# Apply CA defaults (CA uses a descriptive CN, not an IP).
	# Only substitute the CA name when --cn was not explicitly
	# provided — respect user intent.
	if [ "$_cn_explicit" = 'false' ] \
			&& echo "$_cn" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$'; then
		_cn="$_DEFAULT_CA_CN"
	fi
	if [ -z "$_curve" ]; then _curve='P-256'; fi
	if [ -z "$_hash" ]; then _hash='SHA256'; fi

	local _cert_file="${_CERT_DIR}/ca/root.crt"
	local _key_file="${_KEY_DIR}/ca.key"

	if [ "$_force" = 'false' ] && [ -f "$_cert_file" ] \
			&& [ -f "$_key_file" ]; then
		_emit_yaml 'ca' 'ca' "$_cert_file" "$_key_file" \
			"$_curve" "$_hash"
		return 0
	fi

	_ensure_dirs

	local _openssl_curve _openssl_hash
	_openssl_curve="$(_map_curve "$_curve")"
	_openssl_hash="$(_map_hash "$_hash")"

	# Generate CA private key
	openssl ecparam -genkey -name "$_openssl_curve" -noout \
		-out "$_key_file"
	chmod 600 "$_key_file"

	# Generate self-signed CA certificate
	openssl req -new -x509 \
		"$_openssl_hash" \
		-key "$_key_file" \
		-out "$_cert_file" \
		-days "$_days" \
		-subj "/CN=${_cn}"

	_emit_yaml 'ca' 'ca' "$_cert_file" "$_key_file" \
		"$_curve" "$_hash"
}

_gen_server() {
	local _name="$1"; shift
	_validate_name "$_name"
	_parse_opts "$@"

	# Apply server defaults
	if [ -z "$_curve" ]; then _curve='P-256'; fi
	if [ -z "$_hash" ]; then _hash='SHA256'; fi

	local _cert_file="${_CERT_DIR}/server/${_name}.crt"
	local _key_file="${_KEY_DIR}/${_name}.key"
	local _csr_file="${_CSR_DIR}/${_name}.csr"

	if [ "$_force" = 'false' ] && [ -f "$_cert_file" ] \
			&& [ -f "$_key_file" ]; then
		_emit_yaml 'server' "$_name" "$_cert_file" "$_key_file" \
			"$_curve" "$_hash"
		return 0
	fi

	# Ensure CA exists
	if [ ! -f "${_CERT_DIR}/ca/root.crt" ] \
			|| [ ! -f "${_KEY_DIR}/ca.key" ]; then
		echo 'ERROR: CA not found.  Run "gen-tls-cert.sh ca" first.' >&2
		exit 1
	fi

	_ensure_dirs

	local _openssl_curve _openssl_hash
	_openssl_curve="$(_map_curve "$_curve")"
	_openssl_hash="$(_map_hash "$_hash")"

	# Generate server private key
	openssl ecparam -genkey -name "$_openssl_curve" -noout \
		-out "$_key_file"
	chmod 600 "$_key_file"

	# Generate CSR
	openssl req -new \
		"$_openssl_hash" \
		-key "$_key_file" \
		-out "$_csr_file" \
		-subj "/CN=${_cn}"

	# Sign with CA
	openssl x509 -req \
		"$_openssl_hash" \
		-in "$_csr_file" \
		-CA "${_CERT_DIR}/ca/root.crt" \
		-CAkey "${_KEY_DIR}/ca.key" \
		-CAcreateserial \
		-out "$_cert_file" \
		-days "$_days" \
		-extfile <(printf 'subjectAltName=%s\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth' "$_san")

	_emit_yaml 'server' "$_name" "$_cert_file" "$_key_file" \
		"$_curve" "$_hash"
}

_gen_client() {
	local _name="$1"; shift
	_validate_name "$_name"
	_parse_opts "$@"

	# Apply client defaults
	if [ -z "$_curve" ]; then _curve='P-384'; fi
	if [ -z "$_hash" ]; then _hash='SHA384'; fi

	local _cert_file="${_CERT_DIR}/client/${_name}.crt"
	local _key_file="${_KEY_DIR}/${_name}.key"
	local _csr_file="${_CSR_DIR}/${_name}.csr"

	if [ "$_force" = 'false' ] && [ -f "$_cert_file" ] \
			&& [ -f "$_key_file" ]; then
		_emit_yaml 'client' "$_name" "$_cert_file" "$_key_file" \
			"$_curve" "$_hash"
		return 0
	fi

	# Ensure CA exists
	if [ ! -f "${_CERT_DIR}/ca/root.crt" ] \
			|| [ ! -f "${_KEY_DIR}/ca.key" ]; then
		echo 'ERROR: CA not found.  Run "gen-tls-cert.sh ca" first.' >&2
		exit 1
	fi

	_ensure_dirs

	local _openssl_curve _openssl_hash
	_openssl_curve="$(_map_curve "$_curve")"
	_openssl_hash="$(_map_hash "$_hash")"

	# Generate client private key
	openssl ecparam -genkey -name "$_openssl_curve" -noout \
		-out "$_key_file"
	chmod 600 "$_key_file"

	# Generate CSR
	openssl req -new \
		"$_openssl_hash" \
		-key "$_key_file" \
		-out "$_csr_file" \
		-subj "/CN=${_cn}"

	# Sign with CA
	openssl x509 -req \
		"$_openssl_hash" \
		-in "$_csr_file" \
		-CA "${_CERT_DIR}/ca/root.crt" \
		-CAkey "${_KEY_DIR}/ca.key" \
		-CAcreateserial \
		-out "$_cert_file" \
		-days "$_days" \
		-extfile <(printf 'subjectAltName=%s\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=clientAuth' "$_san")

	_emit_yaml 'client' "$_name" "$_cert_file" "$_key_file" \
		"$_curve" "$_hash"
}

_main() {
	if [ $# -lt 1 ]; then
		_usage
	fi

	local _subcmd="$1"; shift

	case "$_subcmd" in
		ca)
			_gen_ca "$@"
			;;
		server)
			if [ $# -lt 1 ]; then
				echo 'ERROR: server subcommand requires a name.' >&2
				_usage
			fi
			_gen_server "$@"
			;;
		client)
			if [ $# -lt 1 ]; then
				echo 'ERROR: client subcommand requires a name.' >&2
				_usage
			fi
			_gen_client "$@"
			;;
		*)
			echo "Unknown subcommand: ${_subcmd}" >&2
			_usage
			;;
	esac
}

_main "$@"
