---
lastmod: '2022-08-27 08:18:19'
title: The role of a data engineer
---

In order to get high-quality and **frequently updated data sets**, it is important to distinguish between data pipelines that are **done and cleaned** by data engineers and all the others that are mostly exploratory. We at Airbus use a folder that is called “cleaned” and all data sets produced there are constantly updated, documented, and of the highest quality. Based on these data sets you create your own. We use the data lake solution [Palantir Foundry](https://en.wikipedia.org/wiki/Palantir_Technologies) (brand name of Airbus: [Skywise](http://www.airbus.com/newsroom/press-releases/en/2017/06/airbus-launches-new-open-aviation-data-platform--skywise--to-sup.html)) which provides you with a map where you see the [data lineage](https://en.wikipedia.org/wiki/Data_lineage) easily. **Documentation and metadata to each data set are crucial** as otherwise, you lose the overview of your data, which is also one main task of a data engineer.

### Services that a data engineer provides

Another important task or **service that a data engineer provides is automation** which data scientists or data analysts do manually. A good overview of what task this includes are provided by <a href="https://medium.com/@maximebeauchemin" target="_blank" rel="noopener">Maxime Beauchemin</a>, the founder of <a href="https://airflow.apache.org/" target="_blank" rel="noopener">Apache Airflow</a>, a tool that helps a data engineer to lift the majority of tasks mentioned:

  * **data ingestion**: services and tooling around “scraping” databases, **loading logs, fetching data from external stores or APIs**, …
  * **metric computation**: frameworks to compute and summarise engagement, **growth or segmentation-related metrics**
  * **anomaly detection**: automating data consumption to **alert people anomalous events occur** or when trends are changing significantly
  * **metadata management**: tooling around allowing generation and consumption of metadata, making it easy to find information in and around the data warehouse
  * **experimentation: [[A-B Testing]]** and experimentation frameworks is often a critical piece of ca ompany’s analytics with a significant data engineering component to it
  * **instrumentation**: **analytics starts with logging events** and attributes related to those events, data engineers have vested interests in making sure that high-quality data is captured upstream
  * **dependencies**: **pipelines that are specialized in understanding series of actions** in time, allowing analysts to understand user behaviors"

> While the nature of the workflows that can be automated differs depending on the environment, the need to automate them is common across the board. By [[Maxime Beauchemin]]

---
References: [[Data Engineering]] [[The role of a data engineer]] [[When is a data engineer needed]]
Tags: #🗃/🌳 #publish