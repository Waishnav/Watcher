
-----------------------
How Watcher works exactly ?
=================

First let's understand how it works.

`./install` file install's all the py scripts and make Watcher to run on startup.

### **v1.0**

- **[afk.py](https://github.com/Waishnav/Watcher/blob/v1.0/src/Watcher/afk.py)** : As name sugested it records the time for being afk it will count afk time when no input is given within 1 min . Also it ignores if their is video playback.

- **[analysis.py](https://github.com/Waishnav/Watcher/blob/v1.0/src/Watcher/analysis.py)** : It gives the report  for the week summary and day summary of the usage.
- **[commands.py](https://github.com/Waishnav/Watcher/blob/v1.0/src/Watcher/commands.py)** : This is used to display the data that analysis.py has analysed. Additionally, it contains all of the command outputs; in essence, this file is where the user interacts with the Wacther.

- **[get_windows.py](https://github.com/Waishnav/Watcher/blob/v1.0/src/Watcher/get_windows.py)** : It gives the title/name of the Active Window/App.

- **[time_operations.py](https://github.com/Waishnav/Watcher/blob/v1.0/src/Watcher/time_operations.py)** : It converts the raw time data into time how much the app is used. Also format produce the time in [HH:MM:SS](https://docs.oracle.com/cd/E41183_01/DR/Time_Formats.html) format.

- **[watch.log](https://github.com/Waishnav/Watcher/blob/v1.0/src/Watcher/watch_log.py)** : This is where the logs file are created it creates two kind of log files for Week analysis and Day analysis, log files for Day analysis are created in [Date].csv format and for week W[week_no.]_[year].py format.


### **v2.0** 

#### Main Difference between [v1.0](https://github.com/Waishnav/Watcher/tree/v1.0/src/Watcher) and [v2.0](https://github.com/Waishnav/Watcher/tree/v2.0/src/Watcher) is the Represntation of the csv and the Algorithm (The way in which it calculate and present the usage time) is **optimized**.

- #### The file which is changed mainly is [analysis.py](https://github.com/Waishnav/Watcher/blob/v2.0/src/Watcher/analysis.py) and the [watch_log.py](https://github.com/Waishnav/Watcher/blob/v2.0/src/Watcher/watch_log.py).

#### In v1.0 their is multiple session of time created even for the same window/apps but in v2.0 this is optimized and been removed the multiple session for each apps.
## Screenshots (csv files)
v1.0            |      v2.0
:-------------------------:|:-------------------------:
![](https://user-images.githubusercontent.com/83799380/194891764-f45c529c-d29d-4d14-96fc-bce0e80becb5.png)  |  ![](https://user-images.githubusercontent.com/83799380/194891867-a49df66a-5ae0-4b9b-9aa7-b6d6681c4574.png)

-----------------------
How can you Contribute to this cool project ?
=================

### Table of Contents

 - [Getting started](https://github.com/Waishnav/Watcher)
 - [Work on Landing page](https://github.com/Waishnav/Watcher/tree/watcher-website)
      - Landing website which helps people to gain clearity about thier screen-time and its importance.
      - It is made with the help of React-framework
 - [Solve v1.0 issues](https://github.com/Waishnav/Watcher/tree/v1.0)
      - There are some bugs in the v1.0 some of them are small some might be big. so you can solve them and improve v1.0.
 - [Improve v2.0](https://github.com/Waishnav/Watcher/tree/v2.0)
      - v2.0 is improvised version of previous algorithm test it and give us feedback in discussion section.
 - [How you can help](#how-you-can-help)
 - [Questions?](#questions)


### How you can help

There are many ways to contribute to Watcher:

 - Work on issues labeled [`good first issue`] or [`help wanted`][help wanted], these are especially suited for new contributors.
 - Fix [`bugs`].
 - Implement new features. NOTE: before implementing and doing PR do discuss with the community with opening issue.
   - Look among the [requested features][requested features] on the forum.
   
 - Write documentation.




## Commit message guidelines

When writing commit messages try to follow [Conventional Commits](https://www.conventionalcommits.org/). It is not a strict requirement (to minimize overhead for new contributors) but it is encouraged.

The format is: 

```
<type>[optional scope]: <description>

[optional body]

[optional footer]
```

Where `type` can be one of: `feat, fix, chore, ci, docs, style, refactor, perf, test`

Examples:

```
 - feat: added ability to sort by duration
 - fix: fixes incorrect week number (#407)
 - docs: improved query documentation 
```


## Questions?

If you have any questions, you can:

 - Create new discussion revolve around your question. [GitHub Discussions](https://github.com/Waishnav/Watcher/discussions).
 - (as a last resort/if needed) Email me (Maintainer): [waishnavdeore@gmail.com](mailto:waishnavdeore@gmail.com)


[github discussions]: https://github.com/Waishnav/Watcher/discussions.
