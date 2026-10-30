# Advisory exceptions need exact RustSec IDs and review reasons.

path:
builtins.all (
  exception:
  builtins.isAttrs exception
  && builtins.isString (exception.id or null)
  && builtins.match "RUSTSEC-[0-9]{4}-[0-9]{4}" exception.id != null

  && builtins.isString (exception.reason or null)
  && exception.reason != ""
) ((builtins.fromTOML (builtins.readFile path)).advisories.ignore or [ ])
