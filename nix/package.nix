{
  lib,
  sqlite,
  naersk',
  src,
  release ? true,
}:
naersk'.buildPackage {
  name = "bp-to-bagels-csv";
  inherit src release;

  nativeBuildInputs = [ sqlite ];

  meta = {
    description = "Import a Banque Populaire CSV export into a Bagels SQLite database.";
    homepage = "https://github.com/wallago/bp-to-bagels-csv";
    license = with lib.licenses; [
      mit
      asl20
    ];
  };
}
