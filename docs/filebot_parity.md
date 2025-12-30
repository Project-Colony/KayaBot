# Spécification de parité FileBot

Ce document définit le périmètre fonctionnel à émuler pour atteindre une parité avec FileBot, en distinguant un MVP des fonctionnalités avancées, et en formalisant la compatibilité attendue.

## 1) Fonctionnalités FileBot à émuler

### Renommage et organisation
- **Renommage de fichiers** selon des règles configurables (pattern-based).
- **Déplacement/organisation** en dossiers (saisons, séries, films, collections).
- **Gestion des collisions** (doublons, noms identiques, suffixes).
- **Conservation/normalisation** des extensions.

### Matching et identification
- **Matching séries TV** (titre, saison/épisode, titre d’épisode, année).
- **Matching films** (titre, année, édition/cut).
- **Matching par hash/empreinte** quand disponible.
- **Résolution d’ambiguïtés** (plusieurs résultats possibles).

### Métadonnées et enrichissement
- **Récupération de métadonnées** (titre officiel, année, résumé, genres).
- **Enrichissement des fichiers** (tags de base et/ou sidecar `.nfo`).
- **Langue/locale** pour les titres et métadonnées.

### Sous-titres
- **Détection/association** des sous-titres existants.
- **Renommage synchronisé** des sous-titres avec la vidéo.
- **Normalisation des suffixes langue** (ex: `.fr`, `.en`, `.multi`).

### Détection automatique
- **Analyse du nom de fichier** pour extraire saison/épisode ou année.
- **Détection du type** (film vs série vs autre) via heuristiques.

### Formatage et règles
- **Templates de renommage** (variables de métadonnées, ex: `{n}`, `{s}`, `{e}`).
- **Règles de fallback** si certaines métadonnées manquent.
- **Prévisualisation** (dry-run) avant application.

### Multi-sources
- **Capacité à interroger plusieurs sources** et sélectionner la meilleure.
- **Stratégie de priorisation** des sources (TV vs Film).

## 2) Priorités (MVP vs avancé)

### MVP (livrable initial)
1. **Matching TV/Film**
   - Titre + saison/épisode + année.
   - Résolution d’ambiguïtés basique.
2. **Normalisation des noms**
   - Parser de noms existants (SxxEyy, 1x02, 2020, etc.).
3. **Renommage**
   - Templates simples pour TV/Film.
   - Dry-run + application.
4. **Organisation**
   - Dossiers par série/saison et films par titre/année.
5. **Sous-titres (basique)**
   - Renommage synchronisé des `.srt/.ass/.sub` existants.

### Avancé (parité étendue)
1. **Matching avancé**
   - Hash/empreinte, gestion d’éditions/cuts.
   - Désambiguïsation interactive/système de score.
2. **Métadonnées enrichies**
   - Résumés, genres, acteurs, collections.
   - Export `.nfo`/tags.
3. **Multi-sources et fallback**
   - Agrégation et priorisation intelligente.
4. **Sous-titres avancés**
   - Téléchargement/alignement/sélection langue.
5. **Règles complexes**
   - Conditions, transformations, exceptions par type.
6. **Détection automatique avancée**
   - Cas ambigus, packs d’épisodes, specials.

## 3) Compatibilité attendue

### Compatibilité des règles de renommage
- **Support minimal des règles FileBot** via un langage de template compatible.
- Variables attendues (exemples) :
  - `{n}` (nom), `{y}` (année), `{s}` (saison), `{e}` (épisode), `{t}` (titre épisode).
  - `{airdate}` (date de diffusion si disponible).
- **Mapping interne** vers la structure de données propre au projet.

### Gestion multi-sources
- **Sources TV** : priorité aux bases de séries.
- **Sources Film** : priorité aux bases de films.
- **Fallback** : si une source échoue, passer à la suivante.

### Comportements attendus
- **Dry-run** identique (pas de modification, sortie de prévisualisation).
- **Règles stables** : mêmes inputs → mêmes outputs.
- **Localisation** : gestion de la langue pour titres et métadonnées.

## 4) Scope fonctionnel (spéc de projet)

Objectif : garantir une expérience de renommage et de matching équivalente aux usages FileBot les plus courants, en priorisant une exécution déterministe et prévisible.

### Hors-scope (initialement)
- UI graphique complète.
- Requêtes réseau en batch trop coûteuses.
- Support total des plugins propriétaires.

### Livrables
- Spécification MVP + avancé.
- Ébauche de mapping de templates.
- Liste des sources de données acceptées.
