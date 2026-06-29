{
  lib,
  sqlite,
  naersk',
  release ? true,
}:
naersk'.buildPackage {
  name = "bp-to-bagels-csv";
  src = ./.;
  inherit release;

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
