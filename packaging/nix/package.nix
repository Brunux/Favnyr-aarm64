{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  makeWrapper,
  copyDesktopItems,
  makeDesktopItem,
  fontconfig,
  wayland,
  libxkbcommon,
  libX11,
  libXcursor,
  libXrandr,
  libXi,
  libGL,
  vulkan-loader,
}:

stdenv.mkDerivation rec {
  pname = "favnyr";
  version = "0.3.4";

  # One release, one architecture. A second architecture cannot be another entry
  # in the flake's system list: each one has its own archive and its own hash, so
  # the url and the hash would both have to be selected from a per-system table
  # here, and `meta.platforms` widened to match.
  src = fetchurl {
    url = "https://github.com/GlitchAwakened/Favnyr/releases/download/v${version}/favnyr-linux-x86_64.tar.gz";
    hash = "sha256-ft0MkM4Dz1Mq+brX2SCHagsAiISuYIuF0v2u7lAhfkk=";
  };

  nativeBuildInputs = [
    autoPatchelfHook
    makeWrapper
    copyDesktopItems
  ];

  buildInputs = [
    stdenv.cc.cc.lib
    fontconfig
    wayland
    libxkbcommon
    libX11
    libXcursor
    libXrandr
    libXi
    libGL
    vulkan-loader
  ];

  sourceRoot = ".";

  desktopItems = [
    (makeDesktopItem {
      name = "favnyr";
      exec = "favnyr";
      icon = "favnyr";
      comment = "A modern and lightweight file manager";
      desktopName = "Favnyr";
      genericName = "File Manager";
      categories = [
        "System"
        "FileManager"
        "Utility"
      ];
    })
  ];

  installPhase = ''
    runHook preInstall

    install -Dm755 favnyr $out/bin/favnyr
    install -Dm644 favnyr.svg $out/share/icons/hicolor/scalable/apps/favnyr.svg

    runHook postInstall
  '';

  postFixup = ''
    wrapProgram $out/bin/favnyr \
      --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [
          wayland
          libxkbcommon
          libGL
          vulkan-loader
          libX11
          libXcursor
          libXrandr
          libXi
          fontconfig
        ]
      }
  '';

  meta = with lib; {
    description = "A modern and lightweight file manager";
    homepage = "https://github.com/GlitchAwakened/Favnyr";
    license = licenses.gpl3Plus;
    # The recipe unpacks a published binary, it does not build from source.
    sourceProvenance = with sourceTypes; [ binaryNativeCode ];
    mainProgram = "favnyr";
    platforms = [ "x86_64-linux" ];
  };
}