# Loads the .env at the repo root, plus two helpers. Usage: . "$(dirname "$0")/env.sh"
# Variables already set in the environment win over .env, so a run can be redirected:
#   KERNEL_DOWNSTREAM_DIR=/tmp/k scripts/build-kernel
HANOIP_ROOT=$(cd "$(dirname "$0")/.." && pwd)
if [ ! -f "$HANOIP_ROOT/.env" ]; then
	echo "Missing $HANOIP_ROOT/.env (copy .env.example and fill it in)." >&2
	exit 1
fi
# Never trace this eval: under sh -x it printed every value of .env, passwords included (docs/PITFALLS.md).
case $- in *x*) set +x; _hanoip_xtrace=1 ;; *) _hanoip_xtrace= ;; esac
eval "$(sed -n -E 's/^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/export \1="${\1:-\2}"/p' "$HANOIP_ROOT/.env")"
[ -z "$_hanoip_xtrace" ] || set -x

# run CMD...: execute, or only print it when DRY_RUN is set.
run() {
	if [ -n "${DRY_RUN:-}" ]; then printf '+ %s\n' "$*"; else "$@"; fi
}

# confirm "question": ask y/N; ASSUME_YES=1 (or DRY_RUN) skips the question.
confirm() {
	[ -n "${ASSUME_YES:-}${DRY_RUN:-}" ] && return 0
	printf '%s [y/N] ' "$1"
	read -r answer
	[ "$answer" = y ] || [ "$answer" = Y ]
}
