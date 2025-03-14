mkdir testdata testdata/{vanilla,yhf,result} -p
cd testdata
curl --skip-existing https://tetr.io/js/tetrio.js -O
curl --skip-existing https://tetr.io/sfx/tetrio.opus.rsd -O
curl --skip-existing https://tetr.io/res/skins/board/generic/board.png -o vanilla/board.png
curl --skip-existing https://tetr.io/res/skins/board/generic/queue.png -o vanilla/queue.png
curl --skip-existing https://tetr.io/res/skins/board/generic/grid.png -o vanilla/grid.png
curl --skip-existing https://tetr.io/res/skins/minos/connected.png -o vanilla/connected.png
curl --skip-existing https://tetr.io/res/skins/ghost/connected.png -o vanilla/connected_ghost.png
curl --skip-existing https://tetr.io/res/skins/minos/tetrio.png -o vanilla/unconnected.png
curl --skip-existing https://tetr.io/res/skins/ghost/tetrio.png -o vanilla/unconnected_ghost.png

curl --skip-existing https://you.have.fail/ed/at/tetrioplus/data/content/skin/penguin_colonel/SHIMMERING_CYCLONE.zip \
  -o yhf/SHIMMERING_CYCLONE.zip
unzip -o yhf/SHIMMERING_CYCLONE.zip -d yhf/shimmering_cyclone/

curl --skip-existing https://you.have.fail/ed/at/tetrioplus/data/content/sfx/Itsmega/BejeweledSR.zip \
  -o yhf/BejeweledSR.zip

curl --skip-existing https://you.have.fail/ed/at/tetrioplus/data/content/skin/Sobsz/rgb_gamer_minos.gif \
  -o yhf/rgb_gamer_minos.gif

curl --skip-existing https://you.have.fail/ed/at/tetrioplus/data/content/unknown/Starcat_JP/Starcats_Cute_Skin_Pack.tpse \
  -o yhf/Starcats_Cute_Skin_Pack.tpse

curl --skip-existing https://you.have.fail/ed/at/tetrioplus/data/content/skin/UniQMG/Concrete.png \
  -o yhf/Concrete.png