---
lastmod: '2022-09-21 06:47:52'
title: Orchestrators
---

If you go one step further, let's say you choose one of the above technologies, you will most probably run into the need to handle intermediate levels in between. For example to prepare, wrangle, clean, copy, etc. the data from one to another system or another format especially if you working with unstructured data as these need to be mingled in a structured way at the end in one or another way. To keep the overview and handle all these challenging tasks, you need an **Orchestrator** and some **cloud-computing frameworks** which I will explain in the two following chapters to complete the full architecture.


Orchestrations are doing these things at heart:
- invokes computation at the right time
- models the dependencies between computations
- tracks what computation ran

Making orchestrators experts on:
- When stuff happens
- when stuff is going wrong
- what it takes to fix the wrong state

Traditional Orchestartos focus on tasks. But newer generations e.g. [[Dagster]] focus on [[Data Assets]] and [[Software-Defined Assets]], which makes scheduling and orchestration much more powerful. For more see [[Dagster#Why it's so powerful]]. It ties in with the [[Modern Data Stack]].

## Tools
  * [Apache Airflow][24] (created in Airbnb)
  * [Luigi][25] (created in Spotify)
  * [Azkaban][26] (created at LinkedIn)
  * [Apache Oozie][27] (for Hadoop systems)
  * [[Dagster]]
  * [[Prefect]]
  - [[Temporal]]

After you choose your group and even the technology you want to go for, you want to have an Orchestrator. **This is one of the most critical tasks** that gets forgotten most of the time.

 [24]: https://airflow.apache.org/
 [25]: https://github.com/spotify/luigi
 [26]: https://azkaban.github.io/
 [27]: http://oozie.apache.org/

Read more on [Data Orchestration Trends: The Shift From Data Pipelines to Data Products | Airbyte](https://airbyte.com/blog/data-orchestration-trends).

## When to use which tools
### Evolution of Tools 
from [[Data Orchestration Trends- The Shift From Data Pipelines to Data Products]]

Traditionally, orchestrators focused mainly on tasks and operations to reliable schedule and workflow computation in the correct sequence. The best example is the first orchestrator out there, [cron](https://en.wikipedia.org/wiki/Cron). Opposite to crontabs, modern tools need to integrate with the Modern Data Stack.

To understand the complete picture, let’s explore where we came from before Airflow and other bespoken orchestrators these days.

1.  In 1987, it started with the **mother of all scheduling** tools, [(Vixie) cron](https://en.wikipedia.org/wiki/Cron)
2.  to more **graphical drag-and-drop** ETL tools around 2000 such as [Oracle OWB](https://en.wikipedia.org/wiki/Oracle_Warehouse_Builder), [SQL Server Integration Services](https://docs.microsoft.com/en-us/sql/integration-services/sql-server-integration-services?view=sql-server-ver15), [Informatica](https://www.informatica.com/) 
3.  to **simple orchestrators** around 2014 with [Apache Airflow](https://airflow.apache.org/), [Luigi](https://github.com/spotify/luigi), [Oozie](https://oozie.apache.org/)
4.  to **modern orchestrators** around 2019 such as [Prefect](https://www.prefect.io/), [Kedro](https://github.com/quantumblacklabs/kedro), [Dagster](https://github.com/dagster-io/dagster/), or [Temporal](https://github.com/temporalio/temporal)

If you are curious and want to see the complete list of tools and frameworks, I suggest you check out the [Awesome Pipeline List](https://github.com/pditommaso/awesome-pipeline#pipeline-frameworks--libraries) on GitHub.

### Which tools
As of [[2022-09-21]]:
- [[Apache Airflow|Airflow]] when you need "dumb" task scheduling only (no data awareness)
- **[[Dagster]]** when you foresee higher-level data engineering problems. Dagster has more abstractions as they grew from first principles with a holistic view in mind from the [very beginning](https://dagster.io/blog/introducing-dagster). They focus heavily on data integrity, testing, idempotency, data assets, etc.
* **[[Prefect]]** if you need a fast and dynamic modern orchestration with a straightforward way to scale out. They recently revamped the prefect core as [Prefect 2.0](https://www.prefect.io/blog/introducing-prefect-2-0/) with a new second-generation orchestration engine called [Orion](https://www.prefect.io/blog/announcing-prefect-orion/). It has several abstractions that make it a swiss army knife for general task management.
	* With the new engine Orion they build in Prefect 2.0, they're very similar to [[Temporal]] and supports fast low latency application orchestration

Also heard from Nick in the podcast [Re-Bundling The Data Stack With Data Orchestration And Software Defined Assets Using Dagster | Data Engineering Podcast](https://www.dataengineeringpodcast.com/dagster-software-defined-assets-data-orchestration-episode-309/).

---
References: [[Python]] [[What is an Orchestrator]] [[Why you need an Orchestrator]] [[Apache Airflow]]
Tags: #🗃/🌳 #🗺 #publish