---
name: Incorrect Sass Output
about: `fugo-sass` and `dart-sass` differ in output or `fugo-sass` reports an error for a valid style sheet
title: ''
labels: bug
assignees: ''

---

**Failing Sass**:
```
a {
  color: red;
}
```

<!-- Showing output from both tools is optional, but does help in debugging -->
**`fugo-sass` Output**:
```
a {
  color: red;
}
```

**`dart-sass` Output**:
```
a {
  color: red;
}
```
