#!/system/bin/sh
# Fixed public observer protocol only; never evaluate request text as shell.
token=$1
scratch=$2
case "$token" in ''|*[!a-f0-9]*) exit 64;; esac
[ "${#token}" -eq 32 ] || exit 64
[ "$scratch" = "/data/local/tmp/rf312-control-$token" ] || exit 64
[ -d "$scratch" ] && [ ! -L "$scratch" ] || exit 64
base="files/$token/"
last=0
cleanup() { rm -f -- "$scratch/out" "$scratch/err" "$scratch/input"; }
trap cleanup EXIT
valid_read() {
  case "$1" in ready|error|stopped) return 0;; esac
  echo "$1" | grep -Eq '^(response-[0-9]{3}\.json|frame-[0-9]{3}\.png)$'
}
valid_exists() {
  case "$1" in ready|error|stopped) return 0;; esac
  echo "$1" | grep -Eq '^response-[0-9]{3}\.json$'
}
perform() {
  case "$op" in
    ping) printf 'rf312-control-ready\n';;
    exists) valid_exists "$name" || return 64
      run-as org.solosoul.rf312observer test -e "$base$name";;
    size|read) valid_read "$name" || return 64
      if [ "$op" = size ]; then
        run-as org.solosoul.rf312observer stat -c %s "$base$name"
      else
        run-as org.solosoul.rf312observer cat "$base$name"
      fi;;
    put) echo "$name" | grep -Eq '^request-[0-9]{3}\.json$' || return 64
      case "$payload" in ''|*[!A-Za-z0-9+/=]*) return 64;; esac
      printf '%s' "$payload" | base64 -d > "$scratch/input" || return 65
      bytes=$(stat -c %s "$scratch/input") || return 65
      [ "$bytes" -le 16384 ] || return 65
      tmp="${name%.json}.tmp"
      run-as org.solosoul.rf312observer sh -c "umask 077; cat > $base$tmp && mv $base$tmp $base$name" < "$scratch/input";;
    stop) [ "$name" = stop ] || return 64
      run-as org.solosoul.rf312observer touch "${base}stop";;
    focus) dumpsys window displays;;
    pid) pidof com.solosoul.app;;
    instrumentation) dumpsys activity instrumentation;;
    app-stop) am force-stop com.solosoul.app;;
    app-start) am start -W -n com.solosoul.app/.MainActivity;;
    close) :;;
    *) return 64;;
  esac
}
while IFS=' ' read -r magic seq op name payload extra; do
  [ "$magic" = RF312B ] && [ -z "$extra" ] || exit 64
  case "$seq" in ''|*[!0-9]*) exit 64;; esac
  [ "${#seq}" -le 5 ] && [ "$seq" -eq "$((last+1))" ] && [ "$seq" -le 16384 ] || exit 64
  last=$seq
  [ "${#payload}" -le 21848 ] || exit 64
  case "$op" in
    exists|size|read|put|stop) :;;
    ping|focus|pid|instrumentation|app-stop|app-start|close) [ "$name" = - ] || exit 64;;
    *) exit 64;;
  esac
  if [ "$op" != put ]; then [ "$payload" = - ] || exit 64; fi
  perform > "$scratch/out" 2> "$scratch/err"
  rc=$?
  out=$(stat -c %s "$scratch/out") || exit 65
  err=$(stat -c %s "$scratch/err") || exit 65
  [ "$out" -le 4194304 ] && [ "$err" -le 65536 ] || exit 65
  printf 'RF312B %s %s %s %s\n' "$seq" "$rc" "$out" "$err"
  printf 'O '
  base64 -w 0 "$scratch/out" || exit 65
  printf '\nE '
  base64 -w 0 "$scratch/err" || exit 65
  printf '\n'
  [ "$op" != close ] || exit 0
done
exit 65
