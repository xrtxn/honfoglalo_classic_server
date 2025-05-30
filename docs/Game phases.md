This has been taken from `modules/triviador.swf:triviador/game/HandleState_*`

## PREPARE GAME
Some window management, preparing the map

## SELECT BASE
### Phases
Checks game phase
#### Phase 0
Announcement banner
#### Phase 1
Base selection dialog
#### Phase 2
Empty
#### Phase 3
- **Long game**: base selection
- **Else** show the randomly selected bases, update score

## SPREADING
### Phases
#### Phase 0
- **Round 1**: Announcement banner
#### Phase 1
Area selection dialog
#### Phase 2
- **player != iam**: Wait for opponent area selection
#### Phase 3
Area selection dialog
#### Phase 4
Show question
#### Phase 5
Empty
#### Phase 6
> [!info]
> This is currently not implemented on the server side.
#### Phase 7
- **mini_phase_num == 3**: Reorders the boxes (relevant if scores became bigger than others)
- **free_area_number <= 3 && Gameround < war round**: Hide other rounds

## FREE AREAS - Fill remaining areas
> Special variable `skiptipwindows` possibly for SEMU
### Phases
#### Phase 0
Announcement banner
#### Phase 1
Show question
#### Phase 3
Possibly used in quick games, looks like announcement
Sets `autoselected` to true
#### Phase 4
Sets `autoselected` to false
Shows tip question
#### Phase 6
`autoselected` check, todo
Score change, reorder

## WAR - Battle
### Phases
#### Phase 0
Announcement banner
- **round == 1**: Show rounds
#### Phase 1
Area selection
#### Phase 2
Empty
#### Phase 3
Ask attacking area
#### Phase 4
Show question
#### Phase 6
Evaluation
#### Phase 10
Show tip question
#### Phase 12
Tip question evaluate
#### Phase 15
Destroy castle tower
#### Phase 17
Build castle tower
#### Phase 19
Add fortress to area
#### Phase 21
Update state

## WAR OVER - If same score, tiebreaker
### Phases
#### Phase 0
Announcement banner
#### Phase 1
Show question
#### Phase 2
Empty
#### Phase 3
Question results

## GAME OVER
### State 16
#### Phase 0
Show end menu
