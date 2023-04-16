Simple excel engine - supports a small subset of functions currently

Project idea inspired by: https://www.youtube.com/watch?v=HCAgvKQDJng

Expects input of the form
```
1|2|3|4
=sum(a1:d1) * a2 / a3|4|5|6
```
This will evaluate to
```
1|2|3|4
6|4|5|6
```

Future extensions

- [] Implement additional function types
- [] Reduce the amount of clone -> potentially bad choice of bigdecimal
- [] Tidy up some of the code