# Bundled fonts

All faces are compiled into the binary (`include_bytes!`), so every machine draws the same.
The OFL 1.1 text is in `LICENSE-FontAwesome.txt` (it applies to every OFL font here with its
own copyright holder and reserved names).

| File | Font | Licence |
|---|---|---|
| ShareTechMono-Regular.ttf | Share Tech Mono (Carrois) | SIL OFL 1.1 |
| ChakraPetch-Bold.ttf | Chakra Petch (Cadson Demak) | SIL OFL 1.1 |
| BebasNeue-Regular.ttf | Bebas Neue (Dharma Type) | SIL OFL 1.1 |
| IMFellEnglishSC.ttf | IM Fell English SC (Igino Marini) | SIL OFL 1.1 |
| Cinzel-Bold.ttf | Cinzel (Natanael Gama) | SIL OFL 1.1 |
| CormorantSC-Bold.ttf | Cormorant SC (Christian Thalmann) | SIL OFL 1.1 |
| fa-solid-900.ttf | Font Awesome Free 6.7.2, solid (Fonticons, Inc.) | SIL OFL 1.1, see `LICENSE-FontAwesome.txt` |

Game faces (Deadlock's Retail Demo, WoW's Friz Quadrata and Morpheus) are never bundled: they
are read from the user's own install at run time, with the faces above standing in.
