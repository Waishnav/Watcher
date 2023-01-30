import './Header.css';
import logo from "./logo.svg"
import up_arrow from "./up-arrow.svg"
import git_img from "./github.svg"
import { useEffect, useState } from "react"

function Header() {
  const [starCount, setStarCount] = useState(0); // initialize starCount to 0
  const [finalCount, setFinalCount] = useState(0);
  useEffect(() => {
    async function getStars() {
            const response = await fetch("https://api.github.com/repos/Waishnav/Watcher");
            const data = await response.json();
            setFinalCount(data.stargazers_count);
    }
    getStars()
  }, [])

  useEffect(() => {
  // simulate incremental effect
    const interval = setInterval(() => {
      if (starCount < finalCount) {
        setStarCount(prevCount => prevCount + 1); // increase starCount by 1
      }
      return  clearInterval(interval); // stop interval when starCount reaches finalCount
    }, 10); // update starCount every 50 milliseconds

  }, [starCount, finalCount]); 

    return (
        <header>
            <div className="branding">
                <img className="logo" src={logo} alt="Watcher-logo" />
                <a className="text" href="https://github.com/Waishnav/Watcher">
                    Watcher
                    <img src={up_arrow} alt="up_arrow" className="arrow" />
                </a>
            </div>
            <a className="secondary-btn" href="https://github.com/waishnav/watcher">
                <img src={git_img} alt='github image' className="h-8" />
                <div style={{width: "60px"}}>STAR US</div>
                <div className='star-count'>{starCount}</div>
            </a>
        </header>
    )
}

export default Header;
