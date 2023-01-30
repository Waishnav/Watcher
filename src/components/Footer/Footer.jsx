import { useEffect, useState } from 'react';
import './Footer.css';
import logo from "./logo.svg"
import up_arrow from "./up-arrow.svg"


function Footer() {
    const [contributors, setContributors] = useState([]) 
    useEffect(() => {
        async function getContributors() {
            const response = await fetch("https://api.github.com/repos/Waishnav/Watcher/contributors");
            const data = await response.json();
            setContributors(data)
        }
        getContributors()
    }, [])
    console.log(contributors)
    return (
        <footer>
            <div></div>
            <div className="footer-container">
                <div className="contribution">
                    <h1 className="">CONTRIBUTORS</h1>
                    <div className="contributors">
                    {contributors.map(contributor => (
                      <a href={contributor.html_url}>
                        <img src={contributor.avatar_url} alt={`${contributor.login}'s avatar`} key={contributor.id} />
                      </a>
                    ))}
                    </div>
                </div>
                <div className='footer-branding'>
                    <div className='branding'>
                        <img src={logo} alt="Watcher-logo" />
                        <a className='text' href="https://github.com/Waishnav/Watcher">Watcher</a>
                    </div>
                    <div className='github-section'>
                        <a className='github' href="https://github.com/Waishnav/Watcher">Github
                            <img className='up-arrow' src={up_arrow} alt="up_arrow" />
                        </a>
                        <a className='repo' href="https://github.com/Waishnav/Watcher">Waishnav/Watcher</a>
                    </div>
                </div>
            </div>
        </footer>
    )
}

export default Footer;
