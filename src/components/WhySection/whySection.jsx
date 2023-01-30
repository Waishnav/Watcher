import "./whySection.css"
import app_usage from "./app-usage.svg"
import less_resources from "./less-resources.svg"
import detailed_analysis from "./detailed-analysis.svg"
import grid from "./bg-grid.png"

import Accordion from "../Accordion/Accordion.jsx"

const sections = [
  {
    title: "Why you should monitor your screen-time ?",
    content: "Monitoring your screen-time can help you understand how you are spending your time on digital devices. It can help you identify patterns of excessive use, such as spending too much time on social media or gaming, and allow you to make adjustments to improve your overall well-being. Studies have shown that excessive screen-time can lead to negative effects on physical and mental health, including poor sleep, eye strain, and increased risk of depression and anxiety. By monitoring your screen-time, you can make informed decisions about how to use digital devices in a way that promotes balance and well-being."
  },
  {
    title: "How you can analyze your productivity time using Watcher ?",
    content: "Watcher allows you to track your usage of different apps and websites, providing detailed information about how much time you spend on each. This information can help you identify which apps and websites are most distracting or unproductive, and which ones are most beneficial for your work or study. By analyzing your productivity time using Watcher, you can make adjustments to your digital habits to optimize your time and increase your productivity. Additionally, Watcher also provides you with the feature of setting the daily time limits for apps, which can help you to stay productive and avoid distractions."
  },
  {
    title: "Is Watcher GUI App ?",
    content: "No. Right now its just CLI tool. Our philosophy is to keep the things less Resources intensive. But We will make GUI in future once we get 300 stars on GitHub"
  },
  {
    title: "How can I view my screen time data and usage statistics in Watcher?",
    content: "Watcher provides you with a user-friendly interface that allows you to view your screen time data and usage statistics. You can view data on your total screen time, usage of different apps and websites, and usage patterns over time."
  },
  {
    title: "Is Watcher a free App?",
    content: "Watcher is a open-source app and it's free to use."
  }
];
const mystyle = {
    width: "210px",
    marginTop: "0px" 
  };

function whySection() {
    return (
        <section className="why-section">
          <h2>Why Watcher?</h2>
          <div className="cards">
            <div className="card">
                <h2>Less Resources</h2>
                <img src={less_resources} alt="" />
            </div>
            <div className="card">
                <h2>Detailed Analysis</h2>
                <ul>
                   <li>Weekly</li>
                    <li>Daily</li>
                </ul>
                <img src={detailed_analysis} style={mystyle} alt="" />
            </div>
            <div className="card">
                <h2>Show App usage</h2>
                <img src={app_usage} alt="" />
            </div>
          </div>
          <img className="grid" draggable={false} src={grid} alt="" />
          <div className="faq">
            <h2>FAQ</h2>
            <Accordion sections={sections}/>
          </div>
        </section>
    )
}

export default whySection;
